/*! A viewer for Betula Behaviour trees.
*/

/*
Goals:
    - We really want the UI to be able to show blackboard values.
    - As well as possibly inner state of nodes.
    - Need to be able to modify the node's configuration with a nice ui.
    - Want the UI to be able to modify the tree, without losing state in the tree.

Thoughts on interaction:
    - We don't want the UI to own the tree.
    - Tree should be able to run quickly in the background.
    - We don't really want the UI to directly access the tree with a mutex.
      Because that'll likely result in contention, as well as a slower running
      tree if we have a UI running.

Future:
    - Tree needs to be able to run in a different process, where we hook
      up the viewer.

*/

/*
To gui:
    - Node State Change
    - Blackboard Value


    - Results from anything sent to the tree.

To tree:
    - Node / blackboard addition
    - Port / Relation changes.
    - Node configuration set.


For nodes:
    Outputs:
        Ports are first.
        Children are second.
    Inputs:
        Parent is 0.
        Ports are second.
*/

// use betula_core::prelude::*;
// , NodeType
use betula_core::{
    BetulaError, BlackboardId, Node, NodeId as BetulaNodeId, NodeType, PortDirection,
};
use serde::{Deserialize, Serialize};

const RELATION_COLOR: Color32 = Color32::from_rgb(0x00, 0xb0, 0xb0);
const UNKNOWN_COLOR: Color32 = Color32::from_rgb(0x80, 0x80, 0x80);

use uuid::Uuid;

pub mod nodes;

use egui_snarl::{
    ui::{PinInfo, SnarlViewer},
    InPin, InPinId, NodeId as SnarlNodeId, OutPin, OutPinId, Snarl,
};

use betula_common::control::InteractionCommand;
use betula_common::{control::TreeClient, TreeSupport};

/// Trait for nodes in the ui.
///
/// It will never be executed, but sharing functionality from Node is
/// useful as it allows reusing the get_config and set_config methods as
/// well as the ports function.
pub trait UiNode: Node {
    fn ui_title(&self) -> String {
        self.node_type().0.clone()
    }

    fn ui_child_range(&self) -> std::ops::Range<usize> {
        0..usize::MAX
    }

    fn ui_config(&mut self, _ui: &mut Ui, _scale: f32) {}

    fn ui_output_port_count(&self) -> usize {
        self.ports()
            .unwrap_or(vec![])
            .iter()
            .filter(|p| p.direction() == PortDirection::Output)
            .count()
    }
    fn ui_input_port_count(&self) -> usize {
        self.ports()
            .unwrap_or(vec![])
            .iter()
            .filter(|p| p.direction() == PortDirection::Input)
            .count()
    }
}

type UiNodeFactory = Box<dyn Fn() -> Box<dyn UiNode>>;
struct UiNodeSupport {
    // node_type: NodeType,
    display_name: String,
    node_factory: UiNodeFactory,
}

struct UiSupport {
    ui: HashMap<NodeType, UiNodeSupport>,
    tree: TreeSupport,
}
impl UiSupport {
    pub fn new() -> Self {
        Self {
            ui: Default::default(),
            tree: Default::default(),
        }
    }
    pub fn add_node_default<T: Node + UiNode + Default + 'static>(&mut self) {
        self.tree.add_node_default::<T>();
        let ui_support = UiNodeSupport {
            // node_type: T::static_type(),
            display_name: T::static_type().0.clone(),
            node_factory: Box::new(|| Box::new(T::default())),
        };
        self.ui.insert(T::static_type(), ui_support);
    }
    pub fn node_types(&self) -> Vec<NodeType> {
        self.ui.keys().cloned().collect()
    }
    pub fn display_name(&self, node_type: &NodeType) -> String {
        if let Some(node_support) = self.ui.get(node_type) {
            node_support.display_name.clone()
        } else {
            "Unknown Node".into()
        }
    }
    pub fn create_ui_node(&self, node_type: &NodeType) -> Result<Box<dyn UiNode>, BetulaError> {
        if let Some(node_support) = self.ui.get(node_type) {
            Ok((node_support.node_factory)())
        } else {
            Err("no ui node support for {node_type:?}".into())
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ViewerNode {
    id: BetulaNodeId,

    #[serde(skip)]
    ui_node: Option<Box<dyn UiNode>>,

    #[serde(skip)]
    children: Vec<Option<BetulaNodeId>>,

    #[serde(skip)]
    children_remote: Vec<BetulaNodeId>,

    #[serde(skip)]
    children_dirty: bool,
}

impl ViewerNode {
    pub fn new(id: BetulaNodeId) -> Self {
        Self {
            id,
            ui_node: None,
            children: vec![],
            children_remote: vec![],
            children_dirty: false,
        }
    }

    pub fn total_outputs(&self) -> usize {
        let output_count = self
            .ui_node
            .as_ref()
            .map(|n| n.ui_output_port_count())
            .unwrap_or(0);
        // println!("Total outputs: {}", output_count + self.children.len());
        output_count + self.children.len()
    }

    pub fn is_child_output(&self, outpin: &OutPinId) -> bool {
        self.pin_to_child(outpin).is_some()
    }

    /// Update the local children list with the bounds and possible new pins.
    #[track_caller]
    fn update_children(&mut self) {
        let allowed = self
            .ui_node
            .as_ref()
            .map(|n| n.ui_child_range())
            .unwrap_or(0..0);
        println!("Allowed: {allowed:?}");
        if self.children.len() < allowed.start {
            // ensure lower bound.
            self.children
                .append(&mut vec![None; allowed.start - self.children.len()]);
        } else {
            // ensure upper bound.
            if self.children.len() < allowed.end {
                if !self.children.last().copied().flatten().is_none() || self.children.is_empty() {
                    self.children.push(None);
                }
            }
        }
        println!("Updated children to: {:?}", self.children);
    }

    /// Map a pin to a child index.
    fn pin_to_child(&self, outpin: &OutPinId) -> Option<usize> {
        let output_count = self
            .ui_node
            .as_ref()
            .map(|n| n.ui_output_port_count())
            .unwrap_or(0);
        if outpin.output < output_count {
            None
        } else {
            Some(outpin.output - output_count)
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.children_dirty
    }
    pub fn set_clean(&mut self) {
        self.children_dirty = false;
    }

    pub fn is_up_to_date(&self) -> bool {
        let ours = self.children.iter().flatten();
        let theirs = self.children_remote.iter();
        ours.eq(theirs)
    }

    /// Disconnect a particular child.
    #[track_caller]
    pub fn child_disconnect(&mut self, outpin: &OutPinId) {
        if let Some(child_index) = self.pin_to_child(outpin) {
            self.children.get_mut(child_index).map(|z| *z = None);
            self.update_children();
            self.children_dirty = true;
        }
    }

    #[track_caller]
    pub fn child_connect(&mut self, our_pin: &OutPinId, node_id: BetulaNodeId) {
        if let Some(child_index) = self.pin_to_child(our_pin) {
            self.children
                .get_mut(child_index)
                .map(|z| *z = Some(node_id));
            self.update_children();
            self.children_dirty = true;
        }
    }

    pub fn desired_children(&self) -> Vec<BetulaNodeId> {
        self.children.iter().cloned().flatten().collect()
    }
    pub fn children(&self) -> Vec<Option<BetulaNodeId>> {
        self.children.clone()
    }

    pub fn update_children_remote(&mut self, children: &[BetulaNodeId]) {
        // This function doesn't preserve gaps atm.
        self.children = children.iter().map(|z| Some(*z)).collect();
        self.children_remote = children.to_vec();
        self.update_children();
        self.children_dirty = true;
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ViewerBlackboard {
    id: BlackboardId,
    // Full, or just a single?
}

#[derive(Serialize, Deserialize, Debug)]
pub enum BetulaViewerNode {
    Node(ViewerNode),
    Blackboard(ViewerBlackboard),
}
use std::collections::HashMap;

pub struct BetulaViewer {
    // Some ui support... for stuff like configs.
    client: Box<dyn TreeClient>,

    node_map: HashMap<BetulaNodeId, SnarlNodeId>,
    snarl_map: HashMap<SnarlNodeId, BetulaNodeId>,
    ui_support: UiSupport,
}

impl BetulaViewer {
    pub fn client(&self) -> &dyn TreeClient {
        &*self.client
    }

    pub fn new(client: Box<dyn TreeClient>) -> Self {
        let mut ui_support = UiSupport::new();
        // ui_support.add_node_default::<betula_core::nodes::SequenceNode>();
        // ui_support.add_node_default::<betula_core::nodes::SelectorNode>();
        // ui_support.add_node_default::<betula_core::nodes::FailureNode>();
        // ui_support.add_node_default::<betula_core::nodes::SuccessNode>();
        ui_support.add_node_default::<betula_common::nodes::DelayNode>();
        // ui_support.add_node_default::<betula_common::nodes::TimeNode>();
        BetulaViewer {
            client,
            ui_support,
            node_map: Default::default(),
            snarl_map: Default::default(),
        }
    }

    pub fn add_id_mapping(&mut self, betula_id: BetulaNodeId, snarl_id: SnarlNodeId) {
        self.node_map.insert(betula_id, snarl_id);
        self.snarl_map.insert(snarl_id, betula_id);
    }

    fn get_snarl_id(&self, node_id: BetulaNodeId) -> Result<SnarlNodeId, BetulaError> {
        self.node_map
            .get(&node_id)
            .ok_or(format!("could not find {node_id:?}").into())
            .copied()
    }
    fn get_betula_id(&self, snarl_id: &SnarlNodeId) -> Result<BetulaNodeId, BetulaError> {
        self.snarl_map
            .get(&snarl_id)
            .ok_or(format!("could not find {snarl_id:?}").into())
            .copied()
    }

    fn remove_betula_id(&mut self, node_id: BetulaNodeId) -> Result<SnarlNodeId, BetulaError> {
        let snarl_id = self
            .node_map
            .remove(&node_id)
            .ok_or::<BetulaError>(format!("could not find {node_id:?}").into())?;
        self.snarl_map.remove(&snarl_id);
        Ok(snarl_id)
    }

    fn get_node_mut<'a>(
        &self,
        node_id: BetulaNodeId,
        snarl: &'a mut Snarl<BetulaViewerNode>,
    ) -> Result<&'a mut ViewerNode, BetulaError> {
        if let Some(snarl_id) = self.node_map.get(&node_id) {
            let node = snarl.get_node_mut(*snarl_id);
            if let Some(viewer_node) = node {
                if let BetulaViewerNode::Node(viewer_node) = viewer_node {
                    return Ok(viewer_node);
                } else {
                    Err(format!("snarl node {node_id:?} is no node").into())
                }
            } else {
                Err(format!("snarl node {node_id:?} cannot be found").into())
            }
        } else {
            Err(format!("node {node_id:?} cannot be found").into())
        }
    }

    fn get_node_ref<'a>(
        &self,
        node_id: BetulaNodeId,
        snarl: &'a Snarl<BetulaViewerNode>,
    ) -> Result<&'a ViewerNode, BetulaError> {
        if let Some(snarl_id) = self.node_map.get(&node_id) {
            let node = snarl.get_node(*snarl_id);
            if let Some(viewer_node) = node {
                if let BetulaViewerNode::Node(viewer_node) = viewer_node {
                    return Ok(viewer_node);
                } else {
                    Err(format!("snarl node {node_id:?} is no node").into())
                }
            } else {
                Err(format!("snarl node {node_id:?} cannot be found").into())
            }
        } else {
            Err(format!("node {node_id:?} cannot be found").into())
        }
    }

    fn child_connections(
        &self,
        node_id: BetulaNodeId,
        snarl: &Snarl<BetulaViewerNode>,
    ) -> Result<Vec<(OutPinId, InPinId)>, BetulaError> {
        let viewer_node = self.get_node_ref(node_id, snarl)?;
        let ui_node = viewer_node
            .ui_node
            .as_ref()
            .ok_or(format!("node is only placeholder"))?;
        let port_output_count = ui_node.ui_output_port_count();
        let snarl_parent = self.get_snarl_id(node_id).unwrap();
        let connected = snarl.out_pins_connected(snarl_parent);
        let mut disconnectables = vec![];
        for p in connected {
            if p.output < port_output_count {
                continue; // blackboard output, skip those.
            }
            let from = snarl.out_pin(p);
            for r in from.remotes {
                disconnectables.push((p, r));
            }
        }
        Ok(disconnectables)
    }

    fn child_connections_desired(
        &self,
        node_id: BetulaNodeId,
        snarl: &Snarl<BetulaViewerNode>,
    ) -> Result<Vec<(OutPinId, InPinId)>, BetulaError> {
        let viewer_node = self.get_node_ref(node_id, snarl)?;
        let ui_node = viewer_node
            .ui_node
            .as_ref()
            .ok_or(format!("node is only placeholder"))?;
        let port_output_count = ui_node.ui_output_port_count();
        let children = viewer_node.children();
        let snarl_parent = self.get_snarl_id(node_id)?;

        let mut v = vec![];
        for (i, conn) in children.iter().enumerate() {
            let port_id = i + port_output_count;
            let from = OutPinId {
                node: snarl_parent,
                output: port_id,
            };
            if let Some(child_node) = conn {
                let snarl_child = self.get_snarl_id(*child_node)?;
                let to = InPinId {
                    node: snarl_child,
                    input: 0,
                };
                v.push((from, to));
            }
        }

        Ok(v)
    }

    #[track_caller]
    pub fn service(&mut self, snarl: &mut Snarl<BetulaViewerNode>) -> Result<(), BetulaError> {
        use betula_common::control::InteractionCommand::RemoveNode;
        use betula_common::control::InteractionEvent::CommandResult;
        use betula_common::control::InteractionEvent::NodeInformation;

        // Check for dirty nodes, and update the snarl state.
        let node_ids = snarl.node_ids().map(|(a, _b)| a).collect::<Vec<_>>();
        for node in node_ids {
            let mut disconnections = vec![];
            let mut connections = vec![];
            if let BetulaViewerNode::Node(node) = &snarl[node] {
                if !node.is_up_to_date() {
                    let cmd = InteractionCommand::set_children(node.id, node.desired_children());
                    self.client.send_command(cmd)?;
                }

                // Now, we need to do snarly things.
                // Lets just disconnect all connections, then reconnect the ones we care about.
                if node.is_dirty() {
                    let mut to_disconnect = self.child_connections(node.id, snarl)?;
                    disconnections.append(&mut to_disconnect);
                    let mut to_connect = self.child_connections_desired(node.id, snarl)?;
                    connections.append(&mut to_connect);
                }
            }
            for (from, to) in disconnections {
                snarl.disconnect(from, to);
            }
            for (from, to) in connections {
                snarl.connect(from, to);
            }
            if let BetulaViewerNode::Node(node) = &mut snarl[node] {
                node.set_clean();
            }
        }

        loop {
            if let Some(event) = self.client.get_event()? {
                println!("event {event:?}");
                match event {
                    NodeInformation(v) => {
                        let viewer_node = self.get_node_mut(v.id, snarl)?;
                        if viewer_node.ui_node.is_none() {
                            viewer_node.ui_node =
                                Some(self.ui_support.create_ui_node(&v.node_type)?);
                            viewer_node.update_children();
                        }
                        viewer_node.update_children_remote(&v.children);
                    }
                    CommandResult(c) => match c.command {
                        RemoveNode(node_id) => {
                            let snarl_id = self.remove_betula_id(node_id)?;
                            snarl.remove_node(snarl_id);
                        }
                        _ => {}
                    },

                    unknown => return Err(format!("Unhandled event {unknown:?}").into()),
                }
            } else {
                break;
            }
        }
        Ok(())
    }

    pub fn ui_create_node(
        &mut self,
        id: BetulaNodeId,
        pos: egui::Pos2,
        node_type: NodeType,
        snarl: &mut Snarl<BetulaViewerNode>,
    ) -> BetulaNodeId {
        let cmd = InteractionCommand::add_node(id, node_type);
        if let Ok(_) = self.client.send_command(cmd) {
            let snarl_id = snarl.insert_node(pos, BetulaViewerNode::Node(ViewerNode::new(id)));
            self.add_id_mapping(id, snarl_id);
        }
        id
    }

    pub fn connect_relation(
        &mut self,
        parent: BetulaNodeId,
        child: BetulaNodeId,
        position: usize,
        snarl: &mut Snarl<BetulaViewerNode>,
    ) -> Result<(), BetulaError> {
        let viewer_node = self.get_node_mut(parent, snarl)?;
        let ui_node = viewer_node.ui_node.as_mut().unwrap();
        let port_output_count = ui_node.ui_output_port_count();
        let output_port = port_output_count + position;
        let from_snarl_id = self.get_snarl_id(parent)?;
        let to_snarl_id = self.get_snarl_id(child)?;
        let from = snarl.out_pin(egui_snarl::OutPinId {
            node: from_snarl_id,
            output: output_port,
        });
        let to = snarl.in_pin(egui_snarl::InPinId {
            node: to_snarl_id,
            input: 0,
        });
        self.connect(&from, &to, snarl);
        Ok(())
    }
}

use egui::{Color32, Ui};

impl SnarlViewer<BetulaViewerNode> for BetulaViewer {
    fn title(&mut self, node: &BetulaViewerNode) -> std::string::String {
        match node {
            BetulaViewerNode::Node(node) => {
                // Grab the type support for this node.
                if let Some(ui_node) = &node.ui_node {
                    ui_node.ui_title()
                } else {
                    "Pending...".to_owned()
                }
            }
            _ => todo!(),
        }
    }

    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<BetulaViewerNode>) {
        // Validate connection
        let to_disconnect;
        let to_connect;
        match (&snarl[from.id.node], &snarl[to.id.node]) {
            (BetulaViewerNode::Node(_), BetulaViewerNode::Blackboard(_)) => {
                // Setup an output port.
                todo!("Setup an output port.")
            }
            (BetulaViewerNode::Node(parent), BetulaViewerNode::Node(child_node)) => {
                if parent.id == child_node.id {
                    println!("Not allow connections to self.");
                    return;
                }
                to_disconnect = Some(to.id);
                to_connect = Some((from.id, to.id));
            }
            (BetulaViewerNode::Blackboard(_), BetulaViewerNode::Node(_)) => {
                // Setup an input port.
                todo!("Setup an input port.")
            }
            (_, _) => {
                // this connection is disallowed.
                return;
            }
        }

        if let Some(to_disconnect) = to_disconnect {
            let pin_with_remotes = snarl.in_pin(to_disconnect);
            if let Some(remote_to_disconnect) = pin_with_remotes.remotes.first() {
                // remote_to_disconnect
                match &mut snarl[to_disconnect.node] {
                    BetulaViewerNode::Node(n) => {
                        n.child_disconnect(&remote_to_disconnect);
                    }
                    _ => unreachable!(),
                }
            }
        }

        if let Some((from, to)) = to_connect {
            match &mut snarl[from.node] {
                BetulaViewerNode::Node(n) => {
                    if let Ok(child_id) = self.get_betula_id(&to.node) {
                        n.child_connect(&from, child_id);
                    }
                }
                _ => unreachable!(),
            }
        }
    }

    fn disconnect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<BetulaViewerNode>) {
        let to_disconnect;
        match (&snarl[from.id.node], &snarl[to.id.node]) {
            (BetulaViewerNode::Node(_), BetulaViewerNode::Node(_)) => {
                to_disconnect = Some(from.id);
            }

            (_, _) => {
                // this connection is disallowed.
                todo!();
            }
        }
        if let Some(to_disconnect) = to_disconnect {
            if let BetulaViewerNode::Node(ref mut node) = &mut snarl[to_disconnect.node] {
                node.child_disconnect(&to_disconnect);
            }
        }
    }

    fn outputs(&mut self, node: &BetulaViewerNode) -> usize {
        match &node {
            BetulaViewerNode::Node(ref node) => node.total_outputs(),
            _ => 0,
        }
    }

    fn vertical_output(
        &mut self,
        pin: &OutPin,
        snarl: &mut Snarl<BetulaViewerNode>,
    ) -> Option<PinInfo> {
        match snarl[pin.id.node] {
            BetulaViewerNode::Node(ref mut node) => {
                if node.is_child_output(&pin.id) {
                    if pin.remotes.is_empty() {
                        Some(
                            PinInfo::triangle()
                                .with_fill(RELATION_COLOR)
                                .vertical()
                                .wiring()
                                .with_gamma(0.5),
                        )
                    } else {
                        Some(PinInfo::triangle().with_fill(RELATION_COLOR).vertical())
                    }
                } else {
                    None
                }
            }
            _ => todo!(),
        }
    }

    fn show_output(
        &mut self,
        pin: &OutPin,
        _: &mut Ui,
        _: f32,
        snarl: &mut Snarl<BetulaViewerNode>,
    ) -> PinInfo {
        match snarl[pin.id.node] {
            BetulaViewerNode::Node(ref _node) => {
                // let child_ports = node.vertical_outputs();
                if pin.remotes.is_empty() {
                    PinInfo::triangle()
                        .with_fill(RELATION_COLOR)
                        .vertical()
                        .wiring()
                        .with_gamma(0.5)
                } else {
                    PinInfo::triangle().with_fill(RELATION_COLOR).vertical()
                }
            }
            _ => todo!(),
        }
    }

    fn inputs(&mut self, node: &BetulaViewerNode) -> usize {
        match &node {
            BetulaViewerNode::Node(ref _node) => 1,
            _ => todo!(),
        }
    }

    fn vertical_input(
        &mut self,
        pin: &InPin,
        snarl: &mut Snarl<BetulaViewerNode>,
    ) -> Option<PinInfo> {
        match snarl[pin.id.node] {
            BetulaViewerNode::Node(ref _node) => {
                if pin.id.input == 0 {
                    Some(PinInfo::triangle().with_fill(RELATION_COLOR).vertical())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn show_input(
        &mut self,
        pin: &InPin,
        _: &mut Ui,
        _: f32,
        snarl: &mut Snarl<BetulaViewerNode>,
    ) -> PinInfo {
        match snarl[pin.id.node] {
            BetulaViewerNode::Node(ref _node) => {
                if pin.id.input == 1 {
                    PinInfo::triangle()
                        .with_fill(RELATION_COLOR)
                        .vertical()
                        .wiring()
                        .with_gamma(0.5)
                } else {
                    PinInfo::triangle().with_fill(RELATION_COLOR).vertical()
                }
            }
            _ => todo!(),
        }
    }

    fn input_color(
        &mut self,
        _: &InPin,
        _: &egui::style::Style,
        _: &mut Snarl<BetulaViewerNode>,
    ) -> Color32 {
        UNKNOWN_COLOR
    }
    fn output_color(
        &mut self,
        _: &OutPin,
        _: &egui::style::Style,
        _: &mut Snarl<BetulaViewerNode>,
    ) -> Color32 {
        UNKNOWN_COLOR
    }

    fn graph_menu(
        &mut self,
        pos: egui::Pos2,
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<BetulaViewerNode>,
    ) {
        ui.label("Add node:");
        for node_type in self.ui_support.node_types() {
            let name = self.ui_support.display_name(&node_type);
            if ui.button(name).clicked() {
                self.ui_create_node(BetulaNodeId(Uuid::new_v4()), pos, node_type, snarl);
                ui.close_menu();
            }
        }
    }

    fn node_menu(
        &mut self,
        node: SnarlNodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<BetulaViewerNode>,
    ) {
        ui.label("Node menu");
        if ui.button("Remove").clicked() {
            match &mut snarl[node] {
                BetulaViewerNode::Node(ref mut node) => {
                    let node_id = node.id;
                    let cmd = InteractionCommand::remove_node(node_id);
                    if let Ok(_) = self.client.send_command(cmd) {}
                }
                _ => todo!(),
            };
            ui.close_menu();
        }
    }

    fn has_footer(&mut self, _node: &BetulaViewerNode) -> bool {
        true
    }
    fn show_footer(
        &mut self,
        node: SnarlNodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        scale: f32,
        snarl: &mut Snarl<BetulaViewerNode>,
    ) {
        match &mut snarl[node] {
            BetulaViewerNode::Node(ref mut node) => {
                node.ui_node.as_mut().map(|e| e.ui_config(ui, scale));
            }
            _ => todo!(),
        };
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use betula_common::control::InProcessControlServer;

    fn make_server_check(
        server: InProcessControlServer,
    ) -> std::thread::JoinHandle<Result<(), betula_core::BetulaError>> {
        use betula_core::basic::BasicTree;
        std::thread::spawn(move || -> Result<(), betula_core::BetulaError> {
            use betula_common::control::TreeServer;
            // use betula_core::Tree;

            use betula_common::control::CommandResult;
            use betula_common::control::InteractionEvent;

            let mut tree = BasicTree::new();
            let mut tree_support = TreeSupport::new();
            tree_support.add_node_default::<betula_core::nodes::SequenceNode>();
            tree_support.add_node_default::<betula_core::nodes::SelectorNode>();
            tree_support.add_node_default::<betula_core::nodes::FailureNode>();
            tree_support.add_node_default::<betula_core::nodes::SuccessNode>();
            tree_support.add_node_default_with_config::<betula_common::nodes::DelayNode, betula_common::nodes::DelayNodeConfig>(
                );
            tree_support.add_node_default::<betula_common::nodes::TimeNode>();
            tree_support.add_node_default::<betula_common::nodes::DelayNode>();
            tree_support.add_value_default::<f64>();

            loop {
                let received = server.get_command();
                if received.is_err() {
                    break;
                }
                let received = received.unwrap();

                if let Some(command) = received {
                    println!("    Executing {command:?}");
                    let r = command.execute(&tree_support, &mut tree);
                    match r {
                        Ok(v) => {
                            for event in v {
                                server.send_event(event)?;
                            }
                        }
                        Err(e) => {
                            server.send_event(InteractionEvent::CommandResult(CommandResult {
                                command: command,
                                error: Some(format!("{e:?}")),
                            }))?;
                        }
                    }
                } else {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
            Ok(())
        })
    }

    #[test]
    fn test_connection() -> Result<(), BetulaError> {
        use betula_common::control::InProcessControl;
        let (server, client) = InProcessControl::new();
        use uuid::uuid;
        let delay1 = BetulaNodeId(uuid!("00000000-0000-0000-0000-ffff00000001"));
        let delay2 = BetulaNodeId(uuid!("00000000-0000-0000-0000-ffff00000002"));
        let delay3 = BetulaNodeId(uuid!("00000000-0000-0000-0000-ffff00000003"));

        let server_thing = make_server_check(server);

        let mut snarl = Snarl::<BetulaViewerNode>::new();
        {
            let mut viewer = BetulaViewer::new(Box::new(client));
            viewer
                .client()
                .send_command(InteractionCommand::tree_call(move |tree| {
                    assert!(tree.nodes().len() == 0);
                    Ok(())
                }))?;
            viewer.ui_create_node(
                delay1,
                egui::pos2(0.0, 0.0),
                betula_common::nodes::DelayNode::static_type(),
                &mut snarl,
            );
            viewer.ui_create_node(
                delay2,
                egui::pos2(0.0, 0.0),
                betula_common::nodes::DelayNode::static_type(),
                &mut snarl,
            );
            viewer.ui_create_node(
                delay3,
                egui::pos2(0.0, 0.0),
                betula_common::nodes::DelayNode::static_type(),
                &mut snarl,
            );
            std::thread::sleep(std::time::Duration::from_millis(50));
            // Verify that the tree now has 3 nodes.
            viewer
                .client()
                .send_command(InteractionCommand::tree_call(move |tree| {
                    assert!(tree.nodes().len() == 3);
                    Ok(())
                }))?;
            // Next, setup relations.
            viewer.service(&mut snarl)?;
            viewer.connect_relation(delay1, delay2, 0, &mut snarl)?;
            std::thread::sleep(std::time::Duration::from_millis(50));
            viewer.service(&mut snarl)?;
            viewer.connect_relation(delay1, delay3, 1, &mut snarl)?;
            std::thread::sleep(std::time::Duration::from_millis(50));
            viewer.service(&mut snarl)?;
            std::thread::sleep(std::time::Duration::from_millis(50));
            // Verify the children of delay 1.
            viewer
                .client()
                .send_command(InteractionCommand::tree_call(move |tree| {
                    println!("testing");
                    assert!(tree.nodes().len() == 3);
                    assert!(tree.children(delay1)? == vec![delay2, delay3]);
                    Ok(())
                }))?;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(server_thing.join().is_ok());
        Ok(())
    }
}
