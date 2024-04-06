/*
Goals:
    - [x] Need to be able to modify the node's configuration with a nice ui.
    - [x] Want the UI to be able to modify the tree, without losing state in the tree.
    - [ ] We really want the UI to be able to show blackboard values.
    - [ ] As well as possibly inner state of nodes.

Thoughts on interaction:
    - [x] We don't want the UI to own the tree.
    - [ ] Tree should be able to run quickly in the background.
    - [x] We don't really want the UI to directly access the tree with a mutex.
      Because that'll likely result in contention, as well as a slower running
      tree if we have a UI running.

Future:
    - [x] Tree needs to be able to run in a different process, where we hook
      up the viewer.


On the UI:
    - What about groups / subtree's? Should we put another level of indirection
      where the viewer shows just a subset of nodes in existance?
    - Subtree's only have one parent, so technically reduce into a single node.


Notes:
    A node is up to date if the remote (server) data matches the client.
    A node is dirty if the snarl state needs updating.

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
        Children are the remainder.
    Inputs:
        Parent is 0.
        Ports are the remainder.
*/

use crate::{UiConfigResponse, UiNode, UiSupport};
use egui::{Color32, Ui};

use betula_core::{
    BetulaError, Blackboard, BlackboardId, BlackboardPort, NodeId as BetulaNodeId, NodePort,
    NodeType, PortConnection, PortName,
};

use betula_common::control::InteractionCommand;
use betula_common::control::TreeClient;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const RELATION_COLOR: Color32 = Color32::from_rgb(0x00, 0xb0, 0xb0);
const BLACKBOARD_COLOR: Color32 = Color32::from_rgb(0xb0, 0x00, 0xb0);
const UNKNOWN_COLOR: Color32 = Color32::from_rgb(0x80, 0x80, 0x80);

use uuid::Uuid;

use egui_snarl::{
    ui::{PinInfo, SnarlViewer},
    InPin, InPinId, NodeId as SnarlNodeId, OutPin, OutPinId, Snarl,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct ViewerNode {
    /// The node id for this element.
    id: BetulaNodeId,

    /// The actual ui node handling.
    #[serde(skip)]
    ui_node: Option<Box<dyn UiNode>>,

    /// Local version of the children.
    ///
    /// Basically the vector of children, empty optionals are empty pins.
    #[serde(skip)]
    children_local: Vec<Option<BetulaNodeId>>,

    /// Remote version of children.
    ///
    /// If this differs from the flattened version of children, the node is
    /// considered not up to date and the server is sent an setChildren
    /// instruction.
    #[serde(skip)]
    children_remote: Vec<BetulaNodeId>,

    /// Denotes whether the node child connections are currently dirty.
    ///
    /// A node is marked dirty if a connect or disconnect has occured that
    /// is not yet represented in the actual snarl state. This is necessary
    /// because the viewer may make many connects / disconnects in a single
    /// cycle, this makes it easier to track the changes happening to the
    /// snarl state.
    #[serde(skip)]
    children_dirty: bool,

    /// Denotes whether the configuration needs to be send to the server.
    ///
    /// If the ui_config returns Changed, this is set the true, it is then
    /// sent to the server. After the server sends back a configuration and
    /// that is set to the node, it is set to false again.
    #[serde(skip)]
    config_needs_send: bool,
}

impl ViewerNode {
    pub fn new(id: BetulaNodeId) -> Self {
        Self {
            id,
            ui_node: None,
            children_local: vec![],
            children_remote: vec![],
            children_dirty: false,
            config_needs_send: false,
        }
    }

    pub fn total_outputs(&self) -> usize {
        let output_count = self
            .ui_node
            .as_ref()
            .map(|n| n.ui_output_port_count())
            .unwrap_or(0);
        output_count + self.children_local.len()
    }

    pub fn total_inputs(&self) -> usize {
        let input_count = self
            .ui_node
            .as_ref()
            .map(|n| n.ui_input_port_count())
            .unwrap_or(0);
        input_count + 1 // +1 for parent.
    }

    pub fn is_child_output(&self, outpin: &OutPinId) -> bool {
        self.pin_to_child(outpin).is_some()
    }

    pub fn is_child_input(&self, inpin: &InPinId) -> bool {
        inpin.input == 0
    }

    /// Update the local children list with the bounds and possible new pins.
    #[track_caller]
    fn update_children_local(&mut self) {
        // This function could do with a test...
        let allowed = self
            .ui_node
            .as_ref()
            .map(|n| n.ui_child_range())
            .unwrap_or(0..0);
        if self.children_local.len() < allowed.start {
            // ensure lower bound.
            self.children_local
                .append(&mut vec![None; allowed.start - self.children_local.len()]);
        } else {
            // ensure upper bound.
            if self.children_local.len() < allowed.end {
                if self.children_local.is_empty() {
                    self.children_local.push(None);
                }
                if !self.children_local.last().copied().flatten().is_none() {
                    self.children_local.push(None);
                }
                // need to drop entries from the rear if there's two none's
                if self.children_local.len() > allowed.start && self.children_local.len() > 2 {
                    let last = self.children_local[self.children_local.len() - 1];
                    let second_last = self.children_local[self.children_local.len() - 2];
                    if last.is_none() && second_last.is_none() {
                        self.children_local.pop();
                    }
                }
            }
        }
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

    fn pin_to_input(&self, input: &InPinId) -> Option<usize> {
        if input.input >= 1 {
            Some(input.input - 1)
        } else {
            None
        }
    }

    fn pin_to_output(&self, output: &OutPinId) -> Option<usize> {
        if output.output >= self.children_local.len() {
            Some(output.output - self.children_local.len())
        } else {
            None
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.children_dirty
    }
    pub fn set_clean(&mut self) {
        self.children_dirty = false;
    }

    pub fn clear_config_needs_send(&mut self) {
        self.config_needs_send = false;
    }

    pub fn set_config_needs_send(&mut self) {
        self.config_needs_send = true;
    }
    pub fn config_needs_send(&self) -> bool {
        self.config_needs_send
    }

    pub fn children_is_up_to_date(&self) -> bool {
        let ours = self.children_local.iter().flatten();
        let theirs = self.children_remote.iter();
        ours.eq(theirs)
    }

    /// Disconnect a particular child based on the provided output pin.
    #[track_caller]
    pub fn child_disconnect(&mut self, outpin: &OutPinId) {
        if let Some(child_index) = self.pin_to_child(outpin) {
            self.children_local.get_mut(child_index).map(|z| *z = None);
            self.children_dirty = true;
        }
    }

    #[track_caller]
    pub fn child_connect(&mut self, our_pin: &OutPinId, node_id: BetulaNodeId) {
        if let Some(child_index) = self.pin_to_child(our_pin) {
            // Enforce that the children vector is long enough to have this pin,
            // otherwise we can't make the connection, this can happen if
            // we move a block of inputs in snarl and have to make connections
            // to pins that are not really in existance yet.
            if child_index >= self.children_local.len() {
                self.children_local.resize(child_index + 1, None);
            }
            self.children_local
                .get_mut(child_index)
                .map(|z| *z = Some(node_id));
            self.children_dirty = true;
        }
    }

    pub fn desired_children(&self) -> Vec<BetulaNodeId> {
        self.children_local.iter().cloned().flatten().collect()
    }
    pub fn children_local(&self) -> Vec<Option<BetulaNodeId>> {
        self.children_local.clone()
    }

    pub fn update_children_remote(&mut self, children: &[BetulaNodeId]) {
        // This function doesn't preserve gaps atm.
        self.children_local = children.iter().map(|z| Some(*z)).collect();
        self.children_remote = children.to_vec();
        self.children_dirty = true;
    }

    pub fn node_port(&self, our_pin: &OutPinId) -> Option<NodePort> {
        let port = self
            .ui_node
            .as_ref()
            .map(|z| z.ui_output_port(our_pin.output))
            .flatten()?;
        Some(port.into_node_port(self.id))
    }
}

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
pub struct ViewerId(pub Uuid);

#[derive(Serialize, Deserialize, Debug)]
pub struct ViewerBlackboard {
    id: BlackboardId,

    viewer_id: ViewerId,

    #[serde(skip)]
    data: Option<BlackboardDataRc>,

    #[serde(skip)]
    is_dirty: bool,

    // #[serde(skip)]
    visible_ports: Option<Vec<PortName>>,
}

impl ViewerBlackboard {
    pub fn new(id: BlackboardId) -> Self {
        Self {
            id,
            viewer_id: ViewerId(Uuid::new_v4()),
            data: None,
            is_dirty: false,
            visible_ports: None,
            // owned_connections: vec![],
        }
    }
    pub fn data(&self) -> Option<Ref<'_, BlackboardData>> {
        self.data.as_ref().map(|z| z.borrow())
    }

    pub fn data_mut(&self) -> Option<RefMut<'_, BlackboardData>> {
        self.data.as_ref().map(|z| z.borrow_mut())
    }

    pub fn inputs(&self) -> usize {
        let z = self
            .visible_ports
            .as_ref()
            .map(|z| z.len())
            .unwrap_or_else(|| {
                self.data
                    .as_ref()
                    .map(|d| d.as_ref().borrow().ports().len())
                    .unwrap_or(0)
            })
            + 1;
        println!("z: {z:?}");
        z
    }

    pub fn outputs(&self) -> usize {
        self.visible_ports
            .as_ref()
            .map(|z| z.len())
            .unwrap_or_else(|| {
                self.data
                    .as_ref()
                    .map(|d| d.as_ref().borrow().ports().len())
                    .unwrap_or(0)
            })
            + 1
    }

    fn port_name(&self, id: usize) -> Option<PortName> {
        self.visible_ports
            .as_ref()
            .map(|z| z.get(id).cloned())
            .unwrap_or_else(|| {
                self.data
                    .as_ref()
                    .map(|d| d.as_ref().borrow().ports().get(id).cloned())
                    .flatten()
            })
    }

    pub fn ui_show_input(&self, input: &InPinId, ui: &mut Ui) -> PinInfo {
        if let Some(name) = self.port_name(input.input) {
            println!("input name: {name:?}");
            ui.label(format!("{:?}", name));
            PinInfo::circle().with_fill(BLACKBOARD_COLOR)
        } else {
            PinInfo::circle()
                .with_fill(BLACKBOARD_COLOR)
                .wiring()
                .with_gamma(0.5)
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }
    pub fn set_clean(&mut self) {
        self.is_dirty = false
    }

    pub fn blackboard_port(&self, node_port: &NodePort, inpin: &InPinId) -> BlackboardPort {
        if let Some(visible_ports) = &self.visible_ports {
            if let Some(this_portname) = visible_ports.get(inpin.input) {
                BlackboardPort::new(self.id, this_portname)
            } else {
                BlackboardPort::new(self.id, &node_port.name())
            }
        } else {
            BlackboardPort::new(self.id, &node_port.name())
        }
    }

    pub fn connect_port(&self, port_connection: &PortConnection) {
        if let Some(mut data) = self.data_mut() {
            data.connections_local.insert(port_connection.clone());
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum BetulaViewerNode {
    Node(ViewerNode),
    Blackboard(ViewerBlackboard),
}

use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;
type BlackboardDataRc = Rc<RefCell<BlackboardData>>;

#[derive(Debug)]
pub struct BlackboardData {
    // id: BlackboardId,
    blackboard: Box<dyn Blackboard>,
    connections_local: std::collections::BTreeSet<PortConnection>,
    connections_remote: std::collections::BTreeSet<PortConnection>,
}
impl BlackboardData {
    pub fn is_up_to_date(&self) -> bool {
        self.connections_local
            .iter()
            .eq(self.connections_remote.iter())
    }
    pub fn local_connected_ports(&self) -> Vec<PortConnection> {
        let additions = self.connections_local.difference(&self.connections_remote);
        additions.cloned().collect()
    }
    pub fn local_disconnected_ports(&self) -> Vec<PortConnection> {
        let removals = self.connections_remote.difference(&self.connections_local);
        removals.cloned().collect()
    }
    pub fn set_connections_remote(&mut self, new_remote: &[PortConnection]) {
        self.connections_remote = new_remote.iter().cloned().collect();
    }
    pub fn ports(&self) -> Vec<PortName> {
        self.blackboard.ports()
    }
}

pub struct BetulaViewer {
    /// Client to interact with the server.
    client: Box<dyn TreeClient>,

    /// Node map to go from BetulaNodeId to SnarlNodeId.
    node_map: HashMap<BetulaNodeId, SnarlNodeId>,

    /// Node map to go from SnarlNodeId to BetulaNodeIs
    snarl_map: HashMap<SnarlNodeId, BetulaNodeId>,

    /// Ui support to create new nodes.
    ui_support: UiSupport,

    /// The actual blackboards.
    blackboards: HashMap<BlackboardId, BlackboardDataRc>,

    // / Mapping between blackboards and snarl ids.
    blackboard_map: HashMap<BlackboardId, HashSet<SnarlNodeId>>,
    blackboard_snarl_map: HashMap<SnarlNodeId, BlackboardId>,
}

impl BetulaViewer {
    pub fn client(&self) -> &dyn TreeClient {
        &*self.client
    }

    pub fn new(client: Box<dyn TreeClient>, ui_support: UiSupport) -> Self {
        BetulaViewer {
            client,
            ui_support,
            node_map: Default::default(),
            snarl_map: Default::default(),
            blackboards: Default::default(),
            blackboard_map: Default::default(),
            blackboard_snarl_map: Default::default(),
        }
    }

    pub fn add_id_mapping(&mut self, betula_id: BetulaNodeId, snarl_id: SnarlNodeId) {
        self.node_map.insert(betula_id, snarl_id);
        self.snarl_map.insert(snarl_id, betula_id);
    }

    pub fn add_blackboard_mapping(&mut self, blackboard_id: BlackboardId, snarl_id: SnarlNodeId) {
        self.blackboard_map
            .entry(blackboard_id)
            .or_default()
            .insert(snarl_id);
        self.blackboard_snarl_map.insert(snarl_id, blackboard_id);
    }

    fn get_node_snarl_id(&self, node_id: BetulaNodeId) -> Result<SnarlNodeId, BetulaError> {
        self.node_map
            .get(&node_id)
            .ok_or(format!("could not find {node_id:?}").into())
            .copied()
    }

    fn get_blackboard_snarl_ids(
        &self,
        blackboard_id: BlackboardId,
    ) -> Result<Vec<SnarlNodeId>, BetulaError> {
        let v = self
            .blackboard_map
            .get(&blackboard_id)
            .ok_or(format!("could not find {blackboard_id:?}"))?;
        Ok(v.iter().cloned().collect())
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

    fn remove_blackboard_id(
        &mut self,
        blackboard_id: BlackboardId,
    ) -> Result<Vec<SnarlNodeId>, BetulaError> {
        let mut snarl_id = self
            .blackboard_map
            .remove(&blackboard_id)
            .ok_or::<BetulaError>(format!("could not find {blackboard_id:?}").into())?;
        for id in &snarl_id {
            self.blackboard_snarl_map.remove(id);
        }
        Ok(snarl_id.drain().collect())
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

    /// Obtain the current snarl connections this node has.
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
        let snarl_parent = self.get_node_snarl_id(node_id).unwrap();
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

    /// Create the desired snarl connections according to the children.
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
        let children = viewer_node.children_local();
        let snarl_parent = self.get_node_snarl_id(node_id)?;

        let mut v = vec![];
        for (i, conn) in children.iter().enumerate() {
            let port_id = i + port_output_count;
            let from = OutPinId {
                node: snarl_parent,
                output: port_id,
            };
            if let Some(child_node) = conn {
                let snarl_child = self.get_node_snarl_id(*child_node)?;
                let to = InPinId {
                    node: snarl_child,
                    input: 0,
                };
                v.push((from, to));
            }
        }

        Ok(v)
    }

    /// Iterate through the nodes, check if their remote and local is in sync, if not send updates to server.
    fn send_changes_to_server(
        &mut self,
        snarl: &mut Snarl<BetulaViewerNode>,
    ) -> Result<(), BetulaError> {
        let node_ids = snarl.node_ids().map(|(a, _b)| a).collect::<Vec<_>>();
        for node in node_ids {
            if let BetulaViewerNode::Node(node) = &snarl[node] {
                if !node.children_is_up_to_date() {
                    let cmd = InteractionCommand::set_children(node.id, node.desired_children());
                    self.client.send_command(cmd)?;
                }
            }
        }

        for blackboard in self.blackboards.values() {
            let blackboard = blackboard.borrow();
            if !blackboard.is_up_to_date() {
                let connect_ports = blackboard.local_connected_ports();
                for connect_port in connect_ports {
                    let cmd = InteractionCommand::connect_port(connect_port);
                    self.client.send_command(cmd)?;
                }
                let disconnect_ports = blackboard.local_disconnected_ports();
                for disconnect_port in disconnect_ports {
                    let cmd = InteractionCommand::disconnect_port(disconnect_port);
                    self.client.send_command(cmd)?;
                }
            }
        }

        Ok(())
    }

    /// Update the snarl state based on the current connections and desired connections.
    fn update_snarl_dirty_nodes(
        &mut self,
        snarl: &mut Snarl<BetulaViewerNode>,
    ) -> Result<(), BetulaError> {
        // Check for dirty nodes, and update the snarl state.
        let node_ids = snarl.node_ids().map(|(a, _b)| a).collect::<Vec<_>>();
        for node in node_ids {
            let mut disconnections = vec![];
            let mut connections = vec![];
            if let BetulaViewerNode::Node(node) = &snarl[node] {
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
                node.update_children_local();
            }
        }
        Ok(())
    }

    /// Update the snarl state based on the current connections and desired connections.
    fn update_snarl_dirty_blackboards(
        &mut self,
        snarl: &mut Snarl<BetulaViewerNode>,
    ) -> Result<(), BetulaError> {
        let _ = snarl;
        Ok(())
    }

    fn send_configs_to_server(
        &mut self,
        snarl: &mut Snarl<BetulaViewerNode>,
    ) -> Result<(), BetulaError> {
        let node_ids = snarl.node_ids().map(|(a, _b)| a).collect::<Vec<_>>();
        for node in node_ids {
            if let BetulaViewerNode::Node(node) = &snarl[node] {
                if node.config_needs_send() {
                    if let Some(ui_node) = &node.ui_node {
                        if let Some(config) = ui_node.get_config()? {
                            // Serialize the configuration.
                            let config = self
                                .ui_support
                                .tree_support()
                                .config_serialize(ui_node.node_type(), &*config)?;
                            // Now send it off!
                            let cmd = InteractionCommand::set_config(node.id, config);
                            self.client.send_command(cmd)?;
                        } else {
                            unreachable!("node reported dirty config but no config returned");
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Service routine to handle communication and update state.
    #[track_caller]
    pub fn service(&mut self, snarl: &mut Snarl<BetulaViewerNode>) -> Result<(), BetulaError> {
        use betula_common::control::InteractionCommand::AddBlackboard;
        use betula_common::control::InteractionCommand::RemoveNode;
        use betula_common::control::InteractionEvent::BlackboardInformation;
        use betula_common::control::InteractionEvent::CommandResult;
        use betula_common::control::InteractionEvent::NodeInformation;

        // First, send changes to the server if necessary.
        self.send_changes_to_server(snarl)?;

        // Process any dirty nodes and update the snarl state.
        self.update_snarl_dirty_nodes(snarl)?;

        // Process any dirty blackboards and update the snarl state.
        self.update_snarl_dirty_blackboards(snarl)?;

        // Check if any configurations need to be sent to the server.
        self.send_configs_to_server(snarl)?;

        // Handle any incoming events.
        loop {
            if let Some(event) = self.client.get_event()? {
                println!("event {event:?}");
                match event {
                    NodeInformation(v) => {
                        let viewer_node = self.get_node_mut(v.id, snarl)?;
                        if viewer_node.ui_node.is_none() {
                            viewer_node.ui_node =
                                Some(self.ui_support.create_ui_node(&v.node_type)?);
                        }

                        // Update the configuration if we have one.
                        let ui_node = viewer_node.ui_node.as_mut().unwrap();
                        // Oh, and set the config if we got one
                        if let Some(config) = v.config {
                            let config =
                                self.ui_support.tree_support().config_deserialize(config)?;
                            ui_node.set_config(&*config)?;
                            viewer_node.clear_config_needs_send();
                        }

                        viewer_node.update_children_remote(&v.children);
                        // Pins may have changed, so we must update the snarl state.
                        // Todo: just this node instead of all of them.
                        self.update_snarl_dirty_nodes(snarl)?;
                    }
                    BlackboardInformation(v) => {
                        if let Some(bb) = self.blackboards.get(&v.id) {
                            // do update things.
                            (*bb).borrow_mut().set_connections_remote(&v.connections);
                        } else {
                            let blackboard = self
                                .ui_support
                                .create_blackboard()
                                .ok_or(format!("could not create blackboard"))?;
                            let rc = Rc::new(RefCell::new(BlackboardData {
                                // id: v.id,
                                blackboard,
                                connections_remote: v.connections.iter().cloned().collect(),
                                connections_local: v.connections.iter().cloned().collect(),
                            }));
                            let cloned_rc = Rc::clone(&rc);
                            self.blackboards.insert(v.id, cloned_rc);
                            // Add the reference to any nodes with this id.
                            let snarl_ids = self.get_blackboard_snarl_ids(v.id)?;
                            for id in snarl_ids {
                                let cloned_rc = Rc::clone(&rc);
                                if let BetulaViewerNode::Blackboard(bb) = &mut snarl[id] {
                                    bb.data = Some(cloned_rc);
                                }
                            }
                            // The actual blackboard doesn't have the ports right now.
                        }
                    }
                    CommandResult(c) => {
                        match c.command {
                            RemoveNode(node_id) => {
                                let snarl_id = self.remove_betula_id(node_id)?;
                                snarl.remove_node(snarl_id);
                            }
                            AddBlackboard(blackboard_id) => {
                                if let Some(failure_reason) = c.error {
                                    // Yikes, it failed, lets clean up the mess and remove the node.
                                    println!("Failed to create blackboard: {failure_reason}, cleaning up.");
                                    let ids = self.remove_blackboard_id(blackboard_id)?;
                                    for snarl_id in ids {
                                        snarl.remove_node(snarl_id);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Spawn a new node and send that the the server.
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

    pub fn ui_create_blackboard(
        &mut self,
        id: BlackboardId,
        pos: egui::Pos2,
        snarl: &mut Snarl<BetulaViewerNode>,
    ) {
        let cmd = InteractionCommand::add_blackboard(id);
        if let Ok(_) = self.client.send_command(cmd) {
            let snarl_id =
                snarl.insert_node(pos, BetulaViewerNode::Blackboard(ViewerBlackboard::new(id)));
            self.add_blackboard_mapping(id, snarl_id);
        }
    }

    #[cfg(test)]
    fn connect_relation(
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
        let from_snarl_id = self.get_node_snarl_id(parent)?;
        let to_snarl_id = self.get_node_snarl_id(child)?;
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
            BetulaViewerNode::Blackboard(bb) => {
                // Grab the type support for this node.
                if bb.data().is_some() {
                    "Blackboard".to_owned()
                } else {
                    "Pending...".to_owned()
                }
            }
        }
    }

    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<BetulaViewerNode>) {
        // Validate connection
        let mut child_to_disconnect = None;
        let mut child_to_connect = None;

        let mut port_to_connect = None;

        match (&snarl[from.id.node], &snarl[to.id.node]) {
            (BetulaViewerNode::Node(node), BetulaViewerNode::Blackboard(bb)) => {
                // Setup an output port.
                if let Some(node_port) = node.node_port(&from.id) {
                    let blackboard_port = bb.blackboard_port(&node_port, &to.id);
                    port_to_connect =
                        Some((to.id.node, PortConnection::new(node_port, blackboard_port)));
                }
            }
            (BetulaViewerNode::Node(parent), BetulaViewerNode::Node(child_node)) => {
                if parent.id == child_node.id {
                    println!("Not allow connections to self.");
                    return;
                }
                child_to_disconnect = Some(to.id);
                child_to_connect = Some((from.id, to.id));
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

        if let Some(to_disconnect) = child_to_disconnect {
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

        if let Some((from, to)) = child_to_connect {
            match &mut snarl[from.node] {
                BetulaViewerNode::Node(n) => {
                    if let Ok(child_id) = self.get_betula_id(&to.node) {
                        n.child_connect(&from, child_id);
                    }
                }
                _ => unreachable!(),
            }
        }

        if let Some((id, port_connection)) = port_to_connect {
            // println!("Connection {from_snarl:?} {from_node_port:?} {to_blackboard_port:?}");
            match &mut snarl[id] {
                BetulaViewerNode::Blackboard(bb) => {
                    bb.connect_port(&port_connection);
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

    fn drop_outputs(&mut self, pin: &OutPin, snarl: &mut Snarl<BetulaViewerNode>) {
        let to_disconnect;
        match &snarl[pin.id.node] {
            BetulaViewerNode::Node(_) => {
                // One outpin can only point at a single thing, so we only have to disconnect one child.
                to_disconnect = Some(pin.id);
            }
            _ => {
                todo!();
            }
        }
        if let Some(to_disconnect) = to_disconnect {
            if let BetulaViewerNode::Node(ref mut node) = &mut snarl[to_disconnect.node] {
                node.child_disconnect(&to_disconnect);
            }
        }
    }

    fn drop_inputs(&mut self, pin: &InPin, snarl: &mut Snarl<BetulaViewerNode>) {
        let to_disconnect;
        match &snarl[pin.id.node] {
            BetulaViewerNode::Node(_) => {
                // need to disconnect multiple, namely all remotes.
                to_disconnect = pin
                    .remotes
                    .iter()
                    .map(|v| snarl.out_pin(*v))
                    .collect::<Vec<_>>();
            }
            _ => {
                todo!();
            }
        }
        for outpin in to_disconnect {
            self.disconnect(&outpin, pin, snarl);
        }
    }

    fn outputs(&mut self, node: &BetulaViewerNode) -> usize {
        match &node {
            BetulaViewerNode::Node(ref node) => node.total_outputs(),
            BetulaViewerNode::Blackboard(ref bb) => bb.outputs(),
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
            _ => None,
        }
    }

    fn show_output(
        &mut self,
        pin: &OutPin,
        ui: &mut Ui,
        _: f32,
        snarl: &mut Snarl<BetulaViewerNode>,
    ) -> PinInfo {
        match snarl[pin.id.node] {
            BetulaViewerNode::Node(ref node) => {
                // let child_ports = node.vertical_outputs();
                if node.is_child_output(&pin.id) {
                    if pin.remotes.is_empty() {
                        PinInfo::triangle()
                            .with_fill(RELATION_COLOR)
                            .vertical()
                            .wiring()
                            .with_gamma(0.5)
                    } else {
                        PinInfo::triangle().with_fill(RELATION_COLOR).vertical()
                    }
                } else {
                    if let Some(ui_node) = &node.ui_node {
                        if let Some(input_port) = node.pin_to_output(&pin.id) {
                            if let Some(port) = ui_node.ui_output_port(input_port) {
                                ui.label(format!(
                                    "{:} [{:?}]",
                                    port.name().as_ref(),
                                    port.port_type()
                                ));
                                PinInfo::triangle().with_fill(BLACKBOARD_COLOR)
                            } else {
                                unreachable!("tried to get pin for input beyond range");
                            }
                        } else {
                            unreachable!("tried to get non input pin");
                        }
                    } else {
                        unreachable!("cant show input for pending node");
                    }
                }
            }
            BetulaViewerNode::Blackboard(ref _bb) => {
                if pin.remotes.is_empty() {
                    PinInfo::circle()
                        .with_fill(BLACKBOARD_COLOR)
                        .wiring()
                        .with_gamma(0.5)
                } else {
                    PinInfo::circle().with_fill(BLACKBOARD_COLOR).wiring()
                }
            }
        }
    }

    fn inputs(&mut self, node: &BetulaViewerNode) -> usize {
        match &node {
            BetulaViewerNode::Node(ref node) => node.total_inputs(),
            BetulaViewerNode::Blackboard(ref bb) => bb.inputs(),
        }
    }

    fn vertical_input(
        &mut self,
        pin: &InPin,
        snarl: &mut Snarl<BetulaViewerNode>,
    ) -> Option<PinInfo> {
        match snarl[pin.id.node] {
            BetulaViewerNode::Node(ref node) => {
                if node.is_child_input(&pin.id) {
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
        ui: &mut Ui,
        _: f32,
        snarl: &mut Snarl<BetulaViewerNode>,
    ) -> PinInfo {
        match snarl[pin.id.node] {
            BetulaViewerNode::Node(ref node) => {
                if node.is_child_input(&pin.id) {
                    PinInfo::triangle().with_fill(RELATION_COLOR).vertical()
                } else {
                    if let Some(ui_node) = &node.ui_node {
                        if let Some(input_port) = node.pin_to_input(&pin.id) {
                            if let Some(port) = ui_node.ui_input_port(input_port) {
                                ui.label(format!(
                                    "{:} [{:?}]",
                                    port.name().as_ref(),
                                    port.port_type()
                                ));
                                PinInfo::triangle().with_fill(BLACKBOARD_COLOR)
                            } else {
                                unreachable!("tried to get pin for input beyond range");
                            }
                        } else {
                            unreachable!("tried to get non input pin");
                        }
                    } else {
                        unreachable!("cant show input for pending node");
                    }
                }
            }
            BetulaViewerNode::Blackboard(ref bb) => bb.ui_show_input(&pin.id, ui),
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
        ui.label("Blackboard:");
        if ui.button("New blackboard").clicked() {
            self.ui_create_blackboard(BlackboardId(Uuid::new_v4()), pos, snarl);
            ui.close_menu();
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
                let r = node.ui_node.as_mut().map(|e| e.ui_config(ui, scale));
                if let Some(response) = r {
                    if response == UiConfigResponse::Changed {
                        node.set_config_needs_send()
                    }
                }
            }
            BetulaViewerNode::Blackboard(_) => {}
        };
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use betula_common::{control::InProcessControlServer, TreeSupport};

    use betula_core::{BetulaError, Node};

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
            let mut ui_support = UiSupport::new();
            // ui_support.add_node_default::<betula_core::nodes::SequenceNode>();
            // ui_support.add_node_default::<betula_core::nodes::SelectorNode>();
            // ui_support.add_node_default::<betula_core::nodes::FailureNode>();
            // ui_support.add_node_default::<betula_core::nodes::SuccessNode>();
            ui_support.add_node_default_with_config::<betula_common::nodes::DelayNode, betula_common::nodes::DelayNodeConfig>(
                );
            // ui_support.add_node_default::<betula_common::nodes::TimeNode>();
            // ui_support.add_node_default::<betula_common::nodes::DelayNode>();
            let mut viewer = BetulaViewer::new(Box::new(client), ui_support);
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
