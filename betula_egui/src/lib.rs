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
    InPin, NodeId as SnarlNodeId, OutPin, Snarl,
};

use betula_common::control::InteractionCommand;
use betula_common::{control::TreeClient, TreeSupport};

#[derive(Serialize, Deserialize)]
pub struct ViewerNode {
    id: BetulaNodeId,

    #[serde(skip)]
    ui_node: Option<Box<dyn UiNode>>,

    #[serde(skip)]
    children: Vec<Option<BetulaNodeId>>,
}

impl ViewerNode {
    pub fn vertical_outputs(&self) -> usize {
        let current = self.children.len();
        let allowed = self
            .ui_node
            .as_ref()
            .map(|n| n.ui_child_range())
            .unwrap_or(0..0);
        if current <= allowed.end {
            current + 1
        } else {
            current
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ViewerBlackboard {
    id: BlackboardId,
    // Full, or just a single?
}

#[derive(Serialize, Deserialize)]
pub enum BetulaViewerNode {
    Node(ViewerNode),
    Blackboard(ViewerBlackboard),
}

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
}

use std::collections::HashMap;

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

pub struct BetulaViewer {
    // Some ui support... for stuff like configs.
    client: Box<dyn TreeClient>,

    node_map: HashMap<BetulaNodeId, SnarlNodeId>,
    ui_support: UiSupport,
}
impl BetulaViewer {
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
        }
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

    pub fn service(&mut self, snarl: &mut Snarl<BetulaViewerNode>) -> Result<(), BetulaError> {
        let event = self.client.get_event()?;
        use betula_common::control::InteractionCommand::RemoveNode;
        use betula_common::control::InteractionEvent::CommandResult;
        use betula_common::control::InteractionEvent::NodeInformation;

        if let Some(event) = event {
            println!("event {event:?}");
            match event {
                NodeInformation(v) => {
                    let viewer_node = self.get_node_mut(v.id, snarl)?;
                    viewer_node.ui_node = Some(self.ui_support.create_ui_node(&v.node_type)?);
                    Ok(())
                }
                CommandResult(c) => {
                    match c.command {
                        RemoveNode(node_id) => {
                            if let Some(snarl_id) = self.node_map.remove(&node_id) {
                                snarl.remove_node(snarl_id);
                            }
                        }
                        _ => {}
                    }
                    Ok(())
                }

                unknown => Err(format!("Unhandled event {unknown:?}").into()),
            }
        } else {
            Ok(())
        }
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

    fn outputs(&mut self, node: &BetulaViewerNode) -> usize {
        match &node {
            BetulaViewerNode::Node(ref node) => {
                // First collect the child ports:
                let child_ports = node.vertical_outputs();
                // Then collect the actual output ports.
                let output_count = node
                    .ui_node
                    .as_ref()
                    .map(|n| n.ui_output_port_count())
                    .unwrap_or(0);
                child_ports + output_count
            }
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
                //node.ui_node.as_mut().map(|e|e.ui_config(ui, scale));
                // So we have outputs for the entire length of children.
                let child_ports = node.vertical_outputs();
                if pin.id.output < child_ports {
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
                let id = BetulaNodeId(Uuid::new_v4());
                let cmd = InteractionCommand::add_node(id, node_type);
                if let Ok(_) = self.client.send_command(cmd) {
                    let snarl_id = snarl.insert_node(
                        pos,
                        BetulaViewerNode::Node(ViewerNode {
                            id,
                            ui_node: None,
                            children: vec![],
                        }),
                    );
                    self.node_map.insert(id, snarl_id);
                }
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
