/*
Goals:
    - [x] Need to be able to modify the node's configuration with a nice ui.
    - [x] Want the UI to be able to modify the tree, without losing state in the tree.
    - [x] We really want the UI to be able to show blackboard values.
    - [ ] As well as possibly inner state of nodes?

Thoughts on interaction:
    - [x] We don't want the UI to own the tree.
    - [x] Tree should be able to run quickly in the background.
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

use crate::{UiConfigResponse, UiNode, UiSupport, UiValue};
use egui::{Color32, Ui};

use betula_core::{
    BetulaError, BlackboardId, BlackboardPort, NodeId as BetulaNodeId, NodePort, NodeType,
    PortConnection, PortName,
};

use betula_common::control::InteractionCommand;
use betula_common::control::TreeClient;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const RELATION_COLOR: Color32 = Color32::from_rgb(0x00, 0xb0, 0xb0);
const BLACKBOARD_COLOR: Color32 = Color32::from_rgb(0xb0, 0xb0, 0xb0);
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

    pub fn output_port_count(&self) -> usize {
        self.ui_node
            .as_ref()
            .map(|n| n.ui_output_port_count())
            .unwrap_or(0)
    }

    pub fn total_outputs(&self) -> usize {
        let output_count = self.output_port_count();
        output_count + self.children_local.len()
    }

    pub fn input_port_count(&self) -> usize {
        self.ui_node
            .as_ref()
            .map(|n| n.ui_input_port_count())
            .unwrap_or(0)
    }
    pub fn total_inputs(&self) -> usize {
        let input_count = self.input_port_count();
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
        // TODO This function could do with a test...
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

    pub fn output_port_to_pin(&self, name: &PortName) -> Option<usize> {
        self.ui_node
            .as_ref()
            .map(|z| z.ui_port_output(name))
            .flatten()
    }
    pub fn input_port_to_pin(&self, name: &PortName) -> Option<usize> {
        self.ui_node
            .as_ref()
            .map(|z| z.ui_port_input(name))
            .flatten()
            .map(|z| z + 1)
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

    pub fn node_output_port(&self, our_pin: &OutPinId) -> Option<NodePort> {
        let port_index = self.pin_to_output(our_pin)?;
        let port = self
            .ui_node
            .as_ref()
            .map(|z| z.ui_output_port(port_index))
            .flatten()?;
        Some(port.into_node_port(self.id))
    }
    pub fn node_input_port(&self, our_pin: &InPinId) -> Option<NodePort> {
        let port_index = self.pin_to_input(our_pin)?;
        let port = self
            .ui_node
            .as_ref()
            .map(|z| z.ui_input_port(port_index))
            .flatten()?;
        Some(port.into_node_port(self.id))
    }
}

// #[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
// pub struct ViewerId(pub Uuid);

use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;
type BlackboardDataRc = Rc<RefCell<BlackboardData>>;

use std::collections::{BTreeMap, BTreeSet};

/// Container for the actual data the blackboard holds.
///
/// This holds the ports that exist on the blackboard, their UiValue and all the
/// connections that exist in the backend that relate to this blackboard.
#[derive(Debug)]
pub struct BlackboardData {
    ui_values: BTreeMap<PortName, Box<dyn UiValue>>,
    connections_local: BTreeSet<PortConnection>,
    connections_remote: BTreeSet<PortConnection>,
}
impl BlackboardData {
    /// Return whether local and remote are identical.
    pub fn is_up_to_date(&self) -> bool {
        self.connections_local == self.connections_remote
    }
    /// Get all local connections.
    pub fn connections(&self) -> Vec<PortConnection> {
        self.connections_remote.iter().cloned().collect()
    }
    /// Get ports that exist in local but not in remote.
    pub fn local_connected_ports(&self) -> Vec<PortConnection> {
        let additions = self.connections_local.difference(&self.connections_remote);
        additions.cloned().collect()
    }
    /// Get ports that exist in remote but not in local.
    pub fn local_disconnected_ports(&self) -> Vec<PortConnection> {
        let removals = self.connections_remote.difference(&self.connections_local);
        removals.cloned().collect()
    }
    /// True if changed.
    pub fn set_connections_remote(&mut self, new_remote: &[PortConnection]) -> bool {
        let new_remote: std::collections::BTreeSet<PortConnection> =
            new_remote.iter().cloned().collect();
        let changed = self.connections_local == new_remote;
        if changed {
            self.connections_remote = new_remote;
        }
        changed
    }
    /// Set the values.
    pub fn set_values(&mut self, values: std::collections::BTreeMap<PortName, Box<dyn UiValue>>) {
        self.ui_values = values;
    }
    /// Return the list of port names.
    pub fn ports(&self) -> Vec<PortName> {
        self.ui_values.keys().cloned().collect()
    }

    /// Render a port name.
    pub fn ui_show_input(&mut self, port: &PortName, ui: &mut Ui, scale: f32) {
        if let Some(ui_value) = self.ui_values.get_mut(port) {
            ui_value.ui(ui, scale);
        }
    }

    /// Return a port name at this index
    pub fn port_name(&self, index: usize) -> Option<PortName> {
        self.ui_values.keys().nth(index).cloned()
    }

    /// Rename a port by changing local connections.
    pub fn rename_port(&mut self, current_portname: &PortName, new_portname: &PortName) {
        self.connections_local = self
            .connections_local
            .iter()
            .cloned()
            .map(|mut x| {
                if x.blackboard.name() == *current_portname {
                    x.blackboard.set_name(new_portname);
                }
                x
            })
            .collect();
    }

    pub fn connect_port(&mut self, connection: &PortConnection) {
        self.connections_local.insert(connection.clone());
    }

    pub fn disconnect_port(&mut self, connection: &PortConnection) {
        self.connections_local.remove(connection);
    }

    pub fn disconnect_node_port(&mut self, node_port: &NodePort) {
        self.connections_local = self
            .connections_local
            .iter()
            .filter(|z| z.node != *node_port)
            .cloned()
            .collect();
    }
}

/// A representation of a blackboard in the viewer.
///
/// One blackboard may have multiple representations.
/// Connections to the blackboard may not be visible, this does not mean
/// they don't exist.
///
/// ViewerBlackboard can decide which ports to show, and also which connections
/// they show to other nodes. The ports shows is always a superset of the ports
/// used by the connections.
#[derive(Serialize, Deserialize, Debug)]
pub struct ViewerBlackboard {
    id: BlackboardId,

    #[serde(skip)]
    data: Option<BlackboardDataRc>,

    #[serde(skip)]
    is_dirty: bool,

    /// The ports that are shown for this blackboard.
    ///
    /// If None, all ports on the blackboard are depicted.
    ports: BTreeMap<PortName, ViewerBlackboardPort>,

    pending_connections: HashSet<PortConnection>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ViewerBlackboardPort {
    connections: BTreeSet<PortConnection>,
    value_editor: bool,
    port_name_editor: Option<String>,
}

impl ViewerBlackboard {
    pub fn new(id: BlackboardId) -> Self {
        Self {
            id,
            data: None,
            is_dirty: false,
            ports: Default::default(),
            pending_connections: Default::default(),
        }
    }
    pub fn data(&self) -> Option<Ref<'_, BlackboardData>> {
        self.data.as_ref().map(|z| z.borrow())
    }

    pub fn data_mut(&self) -> Option<RefMut<'_, BlackboardData>> {
        self.data.as_ref().map(|z| z.borrow_mut())
    }

    pub fn inputs(&self) -> usize {
        self.ports.len() + 1
    }

    pub fn outputs(&self) -> usize {
        self.ports.len()
    }

    fn port_name(&self, id: usize) -> Option<PortName> {
        self.ports.keys().nth(id).cloned()
    }

    pub fn port_to_pin(&self, portname: &PortName) -> Option<usize> {
        self.ports.keys().position(|z| *z == *portname)
    }

    fn rename_port(&mut self, current_portname: &PortName, new_portname: &PortName) {
        // Okay, so we are doing a rename, to do so we must:
        // Move all currently displayed connections with current portname to pending with new port name.
        // Tell the data to rename the blackboard port.
        if let Some(mut data) = self.data_mut() {
            data.rename_port(current_portname, new_portname);
        }
        // we currently lose the actual ViewerBlackboardPort, but that's fine.
        if let Some(old_port_state) = self.ports.get(current_portname) {
            self.pending_connections = old_port_state
                .connections
                .iter()
                .cloned()
                .map(|mut x| {
                    if x.blackboard.name() == *current_portname {
                        x.blackboard.set_name(new_portname);
                    }
                    x
                })
                .collect();
        }
    }

    pub fn ui_show_input(&mut self, input: &InPinId, ui: &mut Ui, scale: f32) -> PinInfo {
        if let Some(name) = self.port_name(input.input) {
            let mut do_rename = None;
            if let Some(bb_port) = self.ports.get_mut(&name) {
                // Show a label if not editing, text edit if we are editing.
                if let Some(ref mut editor_string) = &mut bb_port.port_name_editor {
                    let edit_box = egui::TextEdit::singleline(editor_string)
                        .desired_width(0.0)
                        .clip_text(false);
                    let r = ui.add(edit_box);
                    if r.lost_focus() {
                        // do really smart things to ehm, you know, rename this port on the backend.
                        if name.as_ref() != editor_string {
                            do_rename = Some((name.clone(), PortName(editor_string.clone())));
                        }
                        bb_port.port_name_editor = None;
                    }
                } else {
                    let r = ui.label(format!("{}", name.as_ref()));
                    if r.clicked() {
                        bb_port.port_name_editor = Some(name.clone().into());
                    }
                }
            }
            if let Some((old_name, new_name)) = do_rename {
                self.rename_port(&old_name, &new_name);
            }

            // And actually render the ui node.
            if let Some(data) = self.data.as_ref() {
                data.borrow_mut().ui_show_input(&name, ui, scale);
            }
            PinInfo::circle().with_fill(BLACKBOARD_COLOR)
        } else {
            PinInfo::circle()
                .with_fill(BLACKBOARD_COLOR)
                .with_gamma(0.5)
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }
    pub fn set_clean(&mut self) {
        self.is_dirty = false;
    }
    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }

    pub fn blackboard_input_port(&self, node_port: &NodePort, inpin: &InPinId) -> BlackboardPort {
        if let Some(port_name) = self.port_name(inpin.input) {
            BlackboardPort::new(self.id, &port_name)
        } else {
            BlackboardPort::new(self.id, &node_port.name())
        }
    }

    pub fn blackboard_output_port(&self, outpin: &OutPinId) -> BlackboardPort {
        let port_name = self
            .port_name(outpin.output)
            .expect("cannot get port for non existing output pin");
        BlackboardPort::new(self.id, &port_name)
    }

    pub fn connect_port(&mut self, port_connection: &PortConnection) {
        if let Some(mut data) = self.data_mut() {
            data.connect_port(port_connection);
        }
        self.pending_connections.insert(port_connection.clone());
    }
    pub fn disconnect_port(&mut self, port_connection: &PortConnection) {
        if let Some(mut data) = self.data_mut() {
            data.disconnect_port(port_connection);
        }
    }

    pub fn update_changes(&mut self) {
        // Move from pending to real connections if they became real.
        self.process_pending();
        // Enforce removal of removed entries
        self.drop_removed();
    }

    pub fn process_pending(&mut self) {
        let full_connections = self.data().map(|z| z.connections()).unwrap_or(vec![]);
        // println!("Running process pending, full is {full_connections:?}");
        let mut changed = false;
        for pending in self.pending_connections.drain() {
            if full_connections.contains(&pending) {
                // Ensure that this port exists.
                let v = self
                    .ports
                    .entry(pending.blackboard.name())
                    .or_insert_with(|| ViewerBlackboardPort {
                        connections: Default::default(),
                        value_editor: false,
                        port_name_editor: None,
                    });
                v.connections.insert(pending);
                changed = true;
            }
        }

        if changed {
            self.mark_dirty();
        }
    }

    pub fn drop_removed(&mut self) {
        let existing_ports = self.data().map(|d| d.ports()).unwrap_or(vec![]);
        let mut changed = false;
        // First, remove any elements from self.ports that is not in existing ports.
        for our_port_name in self.ports.keys().cloned().collect::<Vec<_>>() {
            if !existing_ports.contains(&our_port_name) {
                self.ports.remove(&our_port_name);
                changed = true;
            }
        }

        // Then, iterate over all the remaining connections and remove whatever the real
        // blackboard doesn't have.
        let full_connections: BTreeSet<_> = self
            .data()
            .map(|d| d.connections())
            .unwrap_or(vec![])
            .drain(..)
            .collect();
        for our_port_info in self.ports.values_mut() {
            let should_be_pruned: Vec<_> = our_port_info
                .connections
                .difference(&full_connections)
                .cloned()
                .collect();
            for discard in should_be_pruned {
                our_port_info.connections.remove(&discard);
                changed = true;
            }
        }
        if changed {
            self.mark_dirty();
        }
    }

    pub fn connections(&self) -> Vec<PortConnection> {
        self.ports
            .values()
            .map(|z| z.connections.iter())
            .flatten()
            .cloned()
            .collect()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum BetulaViewerNode {
    Node(ViewerNode),
    Blackboard(ViewerBlackboard),
}
impl BetulaViewerNode {
    pub fn output_port_count(&self) -> usize {
        match self {
            BetulaViewerNode::Node(s) => s.output_port_count(),
            BetulaViewerNode::Blackboard(bb) => bb.outputs(),
        }
    }
    pub fn input_port_count(&self) -> usize {
        match self {
            BetulaViewerNode::Node(s) => s.input_port_count(),
            BetulaViewerNode::Blackboard(bb) => bb.inputs(),
        }
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

    /// Mapping between blackboards and snarl ids.
    blackboard_map: HashMap<BlackboardId, HashSet<SnarlNodeId>>,
    blackboard_snarl_map: HashMap<SnarlNodeId, BlackboardId>,

    /// Roots
    tree_roots_local: Vec<BetulaNodeId>,
    /// Roots
    tree_roots_remote: Vec<BetulaNodeId>,
}

impl BetulaViewer {
    pub fn client(&self) -> &dyn TreeClient {
        &*self.client
    }

    pub fn new(client: Box<dyn TreeClient>, ui_support: UiSupport) -> Self {
        BetulaViewer {
            client,
            ui_support,
            tree_roots_local: Default::default(),
            tree_roots_remote: Default::default(),
            node_map: Default::default(),
            snarl_map: Default::default(),
            blackboards: Default::default(),
            blackboard_map: Default::default(),
            blackboard_snarl_map: Default::default(),
        }
    }

    pub fn root_remove(&mut self, node_id: BetulaNodeId) {
        let mut z: HashSet<BetulaNodeId> = self.tree_roots_local.iter().cloned().collect();
        z.remove(&node_id);
        self.tree_roots_local = z.iter().cloned().collect();
    }

    pub fn root_add(&mut self, node_id: BetulaNodeId) {
        let mut z: HashSet<BetulaNodeId> = self.tree_roots_local.iter().cloned().collect();
        z.insert(node_id);
        self.tree_roots_local = z.iter().cloned().collect();
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
        blackboard_id: &BlackboardId,
    ) -> Result<Vec<SnarlNodeId>, BetulaError> {
        let v = self
            .blackboard_map
            .get(&blackboard_id)
            .ok_or(format!("could not find {blackboard_id:?}"))?;
        Ok(v.iter().cloned().collect())
    }

    fn get_blackboard_snarl_connections(
        &self,
        bb_snarl: SnarlNodeId,
        snarl: &Snarl<BetulaViewerNode>,
    ) -> Result<Vec<(OutPinId, InPinId)>, BetulaError> {
        let connections = {
            let bb = self.get_blackboard_ref_snarl(bb_snarl, snarl)?;
            bb.connections()
        };
        let mut desired: Vec<(OutPinId, InPinId)> = vec![];
        // Okay, we have the connections of interest, next up is determining the pin ids for all of that.
        for connection in connections {
            if connection.node.direction() == betula_core::PortDirection::Output {
                // From node to this blackboard.
                let node_id = connection.node.node();
                let snarl_node_id = self.get_node_snarl_id(node_id)?;
                let viewer_node = self.get_node_ref(node_id, snarl)?;
                let name = connection.node.name();
                let port_pin = viewer_node
                    .output_port_to_pin(&name)
                    .ok_or(format!("failed to find port for {name:?}"))?;
                // Okay, we can now assemble the out pin.
                let outpin = OutPinId {
                    node: snarl_node_id,
                    output: port_pin,
                };

                // Now, we just have to determine the input pin at this blackboard.
                let blackboard_node = self.get_blackboard_ref_snarl(bb_snarl, snarl)?;
                let blackboard_name = connection.blackboard.name();
                let inpin = InPinId {
                    node: bb_snarl,
                    input: blackboard_node
                        .port_to_pin(&blackboard_name)
                        .ok_or(format!("failed to find port for {blackboard_name:?}"))?,
                };
                desired.push((outpin, inpin));
            } else {
                // From this blackboard to a node.
                let blackboard_name = connection.blackboard.name();
                let blackboard_node = self.get_blackboard_ref_snarl(bb_snarl, snarl)?;
                let output = blackboard_node
                    .port_to_pin(&blackboard_name)
                    .ok_or(format!("failed to find port for {blackboard_name:?}"))?;
                let outpin = OutPinId {
                    node: bb_snarl,
                    output,
                };

                let node_input_id = connection.node.node();
                let input_node = self.get_node_ref(node_input_id, snarl)?;
                let snarl_node_id = self.get_node_snarl_id(node_input_id)?;
                let node_port_name = connection.node.name();
                let input_pin = input_node
                    .input_port_to_pin(&node_port_name)
                    .ok_or(format!("failed to find port for {node_port_name:?}"))?;
                let inpin = InPinId {
                    node: snarl_node_id,
                    input: input_pin,
                };
                desired.push((outpin, inpin));
            }
        }
        Ok(desired)
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
        self.root_remove(node_id);
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

    fn mark_blackboards_dirty(
        &self,
        blackboard_id: &BlackboardId,
        snarl: &mut Snarl<BetulaViewerNode>,
    ) -> Result<(), BetulaError> {
        let snarl_ids = self.get_blackboard_snarl_ids(blackboard_id)?;
        for id in snarl_ids {
            if let BetulaViewerNode::Blackboard(ref mut bb) = &mut snarl[id] {
                bb.mark_dirty()
            }
        }
        Ok(())
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

    pub fn disconnect_node_port(&mut self, node_port: &NodePort) {
        // Iterate over the blackboards to see if we find one that has
        // this connection.
        for v in self.blackboards.values() {
            let mut borrowed = v.borrow_mut();
            borrowed.disconnect_node_port(node_port);
        }
    }

    fn get_blackboard_ref_snarl<'a>(
        &self,
        snarl_id: SnarlNodeId,
        snarl: &'a Snarl<BetulaViewerNode>,
    ) -> Result<&'a ViewerBlackboard, BetulaError> {
        let node = snarl.get_node(snarl_id);
        if let Some(viewer_node) = node {
            if let BetulaViewerNode::Blackboard(bb) = viewer_node {
                return Ok(bb);
            } else {
                Err(format!("snarl id {snarl_id:?} is no blackboard").into())
            }
        } else {
            Err(format!("snarl id {snarl_id:?} cannot be found").into())
        }
    }

    fn child_port_out(
        &self,
        snarl_id: SnarlNodeId,
        snarl: &Snarl<BetulaViewerNode>,
    ) -> (Vec<(OutPinId, InPinId)>, Vec<(OutPinId, InPinId)>) {
        let is_blackboard = matches!(snarl[snarl_id], BetulaViewerNode::Blackboard(_));
        let port_output_count = snarl[snarl_id].output_port_count();
        let connected = snarl.out_pins_connected(snarl_id);
        let mut children = vec![];
        let mut ports = vec![];
        for p in connected {
            let from = snarl.out_pin(p);
            if p.output < port_output_count || is_blackboard {
                for r in from.remotes {
                    ports.push((p, r));
                }
            } else {
                for r in from.remotes {
                    children.push((p, r));
                }
            }
        }
        (children, ports)
    }

    fn child_port_in(
        &self,
        snarl_id: SnarlNodeId,
        snarl: &Snarl<BetulaViewerNode>,
    ) -> (Vec<(OutPinId, InPinId)>, Vec<(OutPinId, InPinId)>) {
        let is_blackboard = matches!(snarl[snarl_id], BetulaViewerNode::Blackboard(_));
        let connected = snarl.in_pins_connected(snarl_id);
        let mut children = vec![];
        let mut ports = vec![];
        for p in connected {
            let to = snarl.in_pin(p);
            if p.input == 0 && !is_blackboard {
                for from in to.remotes {
                    children.push((from, p));
                }
            } else {
                for from in to.remotes {
                    ports.push((from, p));
                }
            }
        }
        (children, ports)
    }

    /// Obtain the current snarl connections this node has.
    fn child_connections(
        &self,
        snarl_id: SnarlNodeId,
        snarl: &Snarl<BetulaViewerNode>,
    ) -> Result<Vec<(OutPinId, InPinId)>, BetulaError> {
        let (children, _ports) = self.child_port_out(snarl_id, snarl);
        Ok(children)
    }

    fn port_connections(
        &self,
        snarl_id: SnarlNodeId,
        snarl: &Snarl<BetulaViewerNode>,
    ) -> Result<Vec<(OutPinId, InPinId)>, BetulaError> {
        let (_children, mut out_ports) = self.child_port_out(snarl_id, snarl);
        let (_children, mut in_ports) = self.child_port_in(snarl_id, snarl);
        in_ports.append(&mut out_ports);
        Ok(in_ports)
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

    /// Create the desired snarl connections according to the children.
    fn port_connections_desired(
        &self,
        snarl_id: SnarlNodeId,
        snarl: &Snarl<BetulaViewerNode>,
    ) -> Result<Vec<(OutPinId, InPinId)>, BetulaError> {
        self.get_blackboard_snarl_connections(snarl_id, snarl)
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
                let disconnect_ports = blackboard.local_disconnected_ports();
                let cmd =
                    InteractionCommand::port_disconnect_connect(&disconnect_ports, &connect_ports);
                self.client.send_command(cmd)?;
            }
        }

        if self.tree_roots_remote != self.tree_roots_local {
            let roots = self.tree_roots_local.iter().cloned().collect::<Vec<_>>();
            let cmd = InteractionCommand::set_roots(&roots);
            self.client.send_command(cmd)?;
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
            let snarl_id = node;
            if let BetulaViewerNode::Node(node) = &snarl[node] {
                // Now, we need to do snarly things.
                // Lets just disconnect all connections, then reconnect the ones we care about.
                if node.is_dirty() {
                    let mut to_disconnect = self.child_connections(snarl_id, snarl)?;
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
        // Draw lines between appropriate ports.
        // Check for dirty nodes, and update the snarl state.
        let node_ids = snarl.node_ids().map(|(a, _b)| a).collect::<Vec<_>>();
        for snarl_id in node_ids {
            let mut disconnections = vec![];
            let mut connections = vec![];
            if let BetulaViewerNode::Blackboard(bb) = &snarl[snarl_id] {
                // Now, we need to do snarly things.
                // Lets just disconnect all connections, then reconnect the ones we care about.
                if bb.is_dirty() {
                    let mut to_disconnect = self.port_connections(snarl_id, snarl)?;
                    disconnections.append(&mut to_disconnect);
                    let mut to_connect = self.port_connections_desired(snarl_id, snarl)?;
                    connections.append(&mut to_connect);
                }
            }

            for (from, to) in disconnections {
                snarl.disconnect(from, to);
            }
            for (from, to) in connections {
                snarl.connect(from, to);
            }
            if let BetulaViewerNode::Blackboard(bb) = &mut snarl[snarl_id] {
                bb.set_clean();
            }
        }
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
        use betula_common::control::InteractionEvent::TreeRoots;

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
                // println!("event {event:?}");
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
                            {
                                let mut bb = (*bb).borrow_mut();
                                let changed = bb.set_connections_remote(&v.connections);
                                if changed {
                                    self.mark_blackboards_dirty(&v.id, snarl)?;
                                }
                                let ui_values = self.ui_support.create_ui_values(&v.port_values)?;
                                bb.set_values(ui_values);
                            }

                            // Handle any pending connections.
                            for snarl_id in self
                                .blackboard_map
                                .get(&v.id)
                                .ok_or("could not find blackboard id in map")?
                            {
                                if let BetulaViewerNode::Blackboard(ref mut bb) =
                                    &mut snarl[*snarl_id]
                                {
                                    bb.update_changes();
                                }
                            }
                        } else {
                            // Convert the values.
                            let ui_values = self.ui_support.create_ui_values(&v.port_values)?;
                            let rc = Rc::new(RefCell::new(BlackboardData {
                                ui_values,
                                connections_remote: v.connections.iter().cloned().collect(),
                                connections_local: v.connections.iter().cloned().collect(),
                            }));
                            let cloned_rc = Rc::clone(&rc);
                            self.blackboards.insert(v.id, cloned_rc);
                            // Add the reference to any nodes with this id.
                            let snarl_ids = self.get_blackboard_snarl_ids(&v.id)?;
                            for id in snarl_ids {
                                let cloned_rc = Rc::clone(&rc);
                                if let BetulaViewerNode::Blackboard(bb) = &mut snarl[id] {
                                    bb.data = Some(cloned_rc);
                                }
                            }
                            // The actual blackboard doesn't have the ports right now.
                        }
                        self.update_snarl_dirty_blackboards(snarl)?;
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
                    TreeRoots(roots) => {
                        self.tree_roots_remote = roots;
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
        let mut port_to_disconnect = None;

        match (&snarl[from.id.node], &snarl[to.id.node]) {
            (BetulaViewerNode::Node(node), BetulaViewerNode::Blackboard(bb)) => {
                // Setup an output port, node to blackboard, multiple allowed.
                if let Some(node_port) = node.node_output_port(&from.id) {
                    let blackboard_port = bb.blackboard_input_port(&node_port, &to.id);
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
            (BetulaViewerNode::Blackboard(bb), BetulaViewerNode::Node(node)) => {
                // Setup an input port, this requires disconnecting any old ports...
                let blackboard_port = bb.blackboard_output_port(&from.id);
                if let Some(node_port) = node.node_input_port(&to.id) {
                    // Disconnect anytyhing that may be connected to this node port.
                    port_to_disconnect = Some(node_port.clone());
                    port_to_connect = Some((
                        from.id.node,
                        PortConnection::new(node_port, blackboard_port),
                    ));
                }
            }

            (BetulaViewerNode::Blackboard(_), BetulaViewerNode::Blackboard(_)) => {
                // Nothing to do, blackboards can't connect to each other directly.
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

        if let Some(node_port) = port_to_disconnect {
            self.disconnect_node_port(&node_port);
        }
        if let Some((id, port_connection)) = port_to_connect {
            match &mut snarl[id] {
                BetulaViewerNode::Blackboard(bb) => {
                    bb.connect_port(&port_connection);
                }
                _ => unreachable!(),
            }
        }
    }

    fn disconnect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<BetulaViewerNode>) {
        let mut child_to_disconnect = None;
        let mut port_to_disconnect = None;
        match (&snarl[from.id.node], &snarl[to.id.node]) {
            (BetulaViewerNode::Node(_), BetulaViewerNode::Node(_)) => {
                child_to_disconnect = Some(from.id);
            }
            (BetulaViewerNode::Node(node), BetulaViewerNode::Blackboard(bb)) => {
                // Disconnect an output port.
                if let Some(node_port) = node.node_output_port(&from.id) {
                    let blackboard_port = bb.blackboard_input_port(&node_port, &to.id);
                    port_to_disconnect =
                        Some((to.id.node, PortConnection::new(node_port, blackboard_port)));
                }
            }

            (BetulaViewerNode::Blackboard(bb), BetulaViewerNode::Node(node)) => {
                // Disconnect an input port.
                let blackboard_port = bb.blackboard_output_port(&from.id);
                if let Some(node_port) = node.node_input_port(&to.id) {
                    port_to_disconnect = Some((
                        from.id.node,
                        PortConnection::new(node_port, blackboard_port),
                    ));
                }
            }
            (BetulaViewerNode::Blackboard(_), BetulaViewerNode::Blackboard(_)) => {
                // Nothing to do, blackboards can't connect to each other directly.
            }
        }
        if let Some(to_disconnect) = child_to_disconnect {
            if let BetulaViewerNode::Node(ref mut node) = &mut snarl[to_disconnect.node] {
                node.child_disconnect(&to_disconnect);
            }
        }
        if let Some((id, port_connection)) = port_to_disconnect {
            match &mut snarl[id] {
                BetulaViewerNode::Blackboard(bb) => {
                    bb.disconnect_port(&port_connection);
                }
                _ => unreachable!(),
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
            BetulaViewerNode::Blackboard(_) => {
                println!("Not doing anything!");
                return;
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
        scale: f32,
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
            BetulaViewerNode::Blackboard(ref mut bb) => bb.ui_show_input(&pin.id, ui, scale),
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
        match &mut snarl[node] {
            BetulaViewerNode::Node(ref mut node) => {
                let node_id = node.id;
                if ui.button("Remove").clicked() {
                    let cmd = InteractionCommand::remove_node(node_id);
                    let _ = self.client.send_command(cmd);
                    ui.close_menu();
                }
                let mut is_root = self.tree_roots_local.contains(&node_id);
                let r = ui.checkbox(&mut is_root, "Root");
                if r.changed() {
                    if is_root {
                        self.root_add(node_id);
                    } else {
                        self.root_remove(node_id);
                    }
                    ui.close_menu();
                }
            }
            _ => todo!(),
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
