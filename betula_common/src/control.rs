use betula_core::{
    blackboard::{BlackboardId, PortConnection, PortName},
    BetulaError, ExecutionStatus, NodeId, NodeType,
};

pub use crate::tree_support::SerializedBlackboardValues;
use crate::{tree_support::SerializedConfig, tree_support::TreeConfig};

use serde::{Deserialize, Serialize};
// we want asynchronous control & interaction with the tree.

// do we need all of this...? :/... yes... we do lol.

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddNodeCommand {
    pub id: NodeId,
    pub node_type: NodeType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SetChildren {
    pub parent: NodeId,
    pub children: Vec<NodeId>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SetConfigCommand {
    pub id: NodeId,
    pub config: SerializedConfig,
}

pub trait TreeCall: std::fmt::Debug + Send {
    fn clone_boxed(&self) -> Box<dyn TreeCall>;
    fn call(&self, tree: &mut dyn Tree) -> Result<(), BetulaError>;
}

#[derive(Clone)]
pub struct TreeCallWrapper<
    TT: Fn(&mut dyn Tree) -> Result<(), BetulaError> + std::marker::Send + Clone + 'static,
> {
    f: TT,
}
impl<TT: Fn(&mut dyn Tree) -> Result<(), BetulaError> + std::marker::Send + Clone + 'static>
    TreeCallWrapper<TT>
{
    pub fn make(f: TT) -> Box<dyn TreeCall> {
        Box::new(Self { f })
    }
}
impl<TT: Fn(&mut dyn Tree) -> Result<(), BetulaError> + std::marker::Send + Clone + 'static>
    TreeCall for TreeCallWrapper<TT>
{
    fn clone_boxed(&self) -> Box<dyn TreeCall> {
        Box::new(self.clone())
    }
    fn call(&self, tree: &mut dyn Tree) -> Result<(), BetulaError> {
        (self.f)(tree)
    }
}
impl<TT: Fn(&mut dyn Tree) -> Result<(), BetulaError> + std::marker::Send + Clone> std::fmt::Debug
    for TreeCallWrapper<TT>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "TreeCall")
    }
}

impl Clone for Box<dyn TreeCall> {
    fn clone(&self) -> Self {
        (**self).clone_boxed()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PortChanges {
    pub disconnect: Vec<PortConnection>,
    pub connect: Vec<PortConnection>,
}

mod option_duration_serde {
    use super::*;
    use serde::{Deserializer, Serializer};
    pub fn serialize<S>(
        value: &Option<std::time::Duration>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let v = value.as_ref().map(|v| v.as_secs_f64());
        Option::<f64>::serialize(&v, serializer)
    }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<std::time::Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<f64>::deserialize(deserializer).map(|v| v.map(std::time::Duration::from_secs_f64))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RunSettings {
    pub roots: Option<bool>,
    pub specific: Vec<NodeId>,
    #[serde(with = "option_duration_serde")]
    pub interval: Option<std::time::Duration>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum InteractionCommand {
    /// Add a new node.
    AddNode(AddNodeCommand),
    /// Remove a node by id.
    RemoveNode(NodeId),

    /// Set a node's children.
    SetChildren(SetChildren),

    /// Name a node.
    SetNodeName(NodeId, Option<String>),

    /// Add a blackboard
    AddBlackboard(BlackboardId),

    /// Remove a blackboard
    RemoveBlackboard(BlackboardId),

    /// Remove values from a blackboard
    RemoveBlackboardPorts(BlackboardId, Vec<PortName>),

    /// Name a blackboard.
    SetBlackboardName(BlackboardId, Option<String>),

    /// Set a node's configuration.
    SetConfig(SetConfigCommand),

    /// Change ports by disconnecting and connecting them.
    PortDisconnectConnect(PortChanges),

    /// Set the tree roots.
    SetRoots(Vec<NodeId>),

    /// Modify the run settings.
    ///
    /// This is not actually a tree property, but it is a common action so
    /// we want to support it.
    RunSettings(RunSettings),

    /// Clear the entire tree.
    Clear,

    /// Request the tree configuration for serialization.
    RequestTreeConfig,

    /// Load a tree configuration into the tree.
    LoadTreeConfig(TreeConfig),

    /// Reset the nodes in the tree.
    ResetNodes,

    /// Reset the nodes in the tree.
    ResetNode(NodeId),

    /// Set the directory used by the tree.
    SetDirectory(Option<String>),

    /// Call the function on the tree, this _obviously_ only works for the
    /// inter process situation, but it is helpful for unit tests.
    #[serde(skip)]
    TreeCall(Box<dyn TreeCall>),
}

use crate::tree_support::TreeSupport;
use betula_core::Tree;

impl InteractionCommand {
    pub fn add_node(id: NodeId, node_type: NodeType) -> Self {
        InteractionCommand::AddNode(AddNodeCommand { id, node_type })
    }

    pub fn add_blackboard(id: BlackboardId) -> Self {
        InteractionCommand::AddBlackboard(id)
    }

    pub fn remove_blackboard(id: BlackboardId) -> Self {
        InteractionCommand::RemoveBlackboard(id)
    }

    pub fn remove_blackboard_ports(id: BlackboardId, ports: &[PortName]) -> Self {
        InteractionCommand::RemoveBlackboardPorts(id, ports.to_vec())
    }

    pub fn remove_node(id: NodeId) -> Self {
        InteractionCommand::RemoveNode(id)
    }

    pub fn set_node_name(id: NodeId, name: Option<String>) -> Self {
        InteractionCommand::SetNodeName(id, name)
    }

    pub fn reset_nodes() -> Self {
        InteractionCommand::ResetNodes
    }

    pub fn reset_node(id: NodeId) -> Self {
        InteractionCommand::ResetNode(id)
    }
    pub fn set_directory(path: Option<&std::path::Path>) -> Self {
        let path = path.map(|v| {
            v.canonicalize()
                .unwrap_or_else(|_| panic!("path {path:?} could not be made canonical"))
                .into_os_string()
                .into_string()
                .unwrap_or_else(|_| panic!("path {path:?} could not be made made into string"))
        });
        InteractionCommand::SetDirectory(path)
    }

    pub fn connect_port(port_connection: PortConnection) -> Self {
        Self::port_disconnect_connect(&[], &[port_connection])
    }
    pub fn disconnect_port(port_connection: PortConnection) -> Self {
        Self::port_disconnect_connect(&[port_connection], &[])
    }

    pub fn port_disconnect_connect(
        disconnect: &[PortConnection],
        connect: &[PortConnection],
    ) -> Self {
        InteractionCommand::PortDisconnectConnect(PortChanges {
            disconnect: disconnect.to_vec(),
            connect: connect.to_vec(),
        })
    }

    pub fn set_roots(ids: &[NodeId]) -> Self {
        InteractionCommand::SetRoots(ids.to_vec())
    }

    pub fn set_children(parent: NodeId, children: Vec<NodeId>) -> Self {
        InteractionCommand::SetChildren(SetChildren { parent, children })
    }
    pub fn tree_call<
        F: Fn(&mut dyn Tree) -> Result<(), BetulaError> + std::marker::Send + Clone + 'static,
    >(
        f: F,
    ) -> Self {
        InteractionCommand::TreeCall(TreeCallWrapper::make(f))
    }

    pub fn set_config(id: NodeId, config: SerializedConfig) -> Self {
        InteractionCommand::SetConfig(SetConfigCommand { id, config })
    }

    pub fn set_blackboard_name(id: BlackboardId, name: Option<String>) -> Self {
        InteractionCommand::SetBlackboardName(id, name)
    }

    pub fn run_specific(nodes: &[NodeId]) -> Self {
        InteractionCommand::RunSettings(RunSettings {
            roots: None,
            interval: None,
            specific: nodes.to_vec(),
        })
    }
    pub fn request_tree_config() -> Self {
        InteractionCommand::RequestTreeConfig
    }
    pub fn load_tree_config(config: TreeConfig) -> Self {
        InteractionCommand::LoadTreeConfig(config)
    }

    fn node_information(
        tree_support: &TreeSupport,
        node_id: NodeId,
        tree: &mut dyn Tree,
    ) -> Result<NodeInformation, BetulaError> {
        let name = tree.node_name(node_id)?;
        let node = tree
            .node_mut(node_id)
            .ok_or(format!("cannot find {node_id:?}"))?;
        let node_type = node.node_type().clone();
        let node_config = node.get_config()?;
        let config = if let Some(node_config) = node_config {
            Some(tree_support.config_serialize(node_type.clone(), &*node_config)?)
        } else {
            None
        };
        let children = tree.children(node_id)?;
        Ok(NodeInformation {
            id: node_id,
            node_type,
            config,
            children,
            name,
        })
    }

    pub fn blackboard_information(
        tree_support: &TreeSupport,
        blackboard_id: BlackboardId,
        tree: &dyn Tree,
    ) -> Result<BlackboardInformation, BetulaError> {
        let bb = tree
            .blackboard_ref(blackboard_id)
            .ok_or(format!("cannot find {blackboard_id:?}"))?;
        let bb = bb.borrow_mut();
        let port_values = tree_support.blackboard_value_serialize(&**bb)?;
        let connections = tree.blackboard_connections(blackboard_id);
        let name = tree.blackboard_name(blackboard_id)?;
        Ok(BlackboardInformation {
            id: blackboard_id,
            port_values,
            connections,
            name,
        })
    }

    pub fn execute(
        &self,
        tree_support: &TreeSupport,
        tree: &mut dyn Tree,
    ) -> Result<Vec<InteractionEvent>, BetulaError> {
        match self {
            InteractionCommand::AddNode(v) => {
                let new_node = tree_support.create_node(&v.node_type)?;
                tree.add_node_boxed(v.id, new_node)?;
                Ok(vec![
                    InteractionEvent::CommandResult(CommandResult {
                        command: self.clone(),
                        error: None,
                    }),
                    InteractionEvent::NodeInformation(Self::node_information(
                        tree_support,
                        v.id,
                        tree,
                    )?),
                ])
            }
            InteractionCommand::SetChildren(v) => {
                tree.set_children(v.parent, &v.children)?;
                Ok(vec![
                    InteractionEvent::CommandResult(CommandResult {
                        command: self.clone(),
                        error: None,
                    }),
                    InteractionEvent::NodeInformation(Self::node_information(
                        tree_support,
                        v.parent,
                        tree,
                    )?),
                ])
            }
            InteractionCommand::RemoveNode(v) => {
                tree.remove_node(*v)?;
                Ok(vec![InteractionEvent::CommandResult(CommandResult {
                    command: self.clone(),
                    error: None,
                })])
            }
            InteractionCommand::SetNodeName(node_id, name) => {
                tree.set_node_name(*node_id, name.as_deref())?;
                Ok(vec![
                    InteractionEvent::CommandResult(CommandResult {
                        command: self.clone(),
                        error: None,
                    }),
                    InteractionEvent::NodeInformation(Self::node_information(
                        tree_support,
                        *node_id,
                        tree,
                    )?),
                ])
            }
            InteractionCommand::AddBlackboard(v) => {
                let blackboard = tree_support
                    .create_blackboard()
                    .ok_or("cannot create blackboard, no factory".to_string())?;
                tree.add_blackboard_boxed(*v, blackboard)?;
                Ok(vec![
                    InteractionEvent::CommandResult(CommandResult {
                        command: self.clone(),
                        error: None,
                    }),
                    InteractionEvent::BlackboardInformation(Self::blackboard_information(
                        tree_support,
                        *v,
                        tree,
                    )?),
                ])
            }
            InteractionCommand::RemoveBlackboard(v) => {
                tree.remove_blackboard(*v)?;
                Ok(vec![InteractionEvent::CommandResult(CommandResult {
                    command: self.clone(),
                    error: None,
                })])
            }
            InteractionCommand::RemoveBlackboardPorts(blackboard_id, ports) => {
                let bb = tree
                    .blackboard_mut(*blackboard_id)
                    .ok_or(format!("cannot find blackboard {blackboard_id:?}"))?;
                for port in ports {
                    bb.remove(port);
                }
                Ok(vec![
                    InteractionEvent::CommandResult(CommandResult {
                        command: self.clone(),
                        error: None,
                    }),
                    InteractionEvent::BlackboardInformation(Self::blackboard_information(
                        tree_support,
                        *blackboard_id,
                        tree,
                    )?),
                ])
            }
            InteractionCommand::PortDisconnectConnect(port_changes) => {
                let mut involved_blackboards: std::collections::HashSet<BlackboardId> =
                    Default::default();
                for port_connection in &port_changes.disconnect {
                    tree.disconnect_port(port_connection)?;
                    involved_blackboards.insert(port_connection.blackboard_id());
                }
                for port_connection in &port_changes.connect {
                    tree.connect_port(port_connection)?;
                    involved_blackboards.insert(port_connection.blackboard_id());
                }
                // Create the interaction events.
                let mut reply = vec![InteractionEvent::CommandResult(CommandResult {
                    command: self.clone(),
                    error: None,
                })];

                for blackboard_id in involved_blackboards {
                    reply.push(InteractionEvent::BlackboardInformation(
                        Self::blackboard_information(tree_support, blackboard_id, tree)?,
                    ));
                }
                Ok(reply)
            }
            InteractionCommand::SetConfig(config_cmd) => {
                // get the node
                let node_id = config_cmd.id;
                let node_mut = tree
                    .node_mut(node_id)
                    .ok_or(format!("cannot find {node_id:?}"))?;
                // deserialize this config.
                let real_config = tree_support.config_deserialize(config_cmd.config.clone())?;
                // And set the config.
                node_mut.set_config(&*real_config)?;
                Ok(vec![
                    InteractionEvent::CommandResult(CommandResult {
                        command: self.clone(),
                        error: None,
                    }),
                    InteractionEvent::NodeInformation(Self::node_information(
                        tree_support,
                        node_id,
                        tree,
                    )?),
                ])
            }
            InteractionCommand::SetRoots(roots) => {
                tree.set_roots(roots)?;
                Ok(vec![
                    InteractionEvent::CommandResult(CommandResult {
                        command: self.clone(),
                        error: None,
                    }),
                    InteractionEvent::TreeRoots(TreeRoots {
                        roots: tree.roots(),
                    }),
                ])
            }
            InteractionCommand::RunSettings(run_settings) => {
                for z in &run_settings.specific {
                    let _result = tree.execute(*z)?;
                }
                Ok(vec![InteractionEvent::CommandResult(CommandResult {
                    command: self.clone(),
                    error: None,
                })])
            }
            InteractionCommand::RequestTreeConfig => {
                let config = tree_support.export_tree_config(tree)?;
                Ok(vec![
                    InteractionEvent::CommandResult(CommandResult {
                        command: self.clone(),
                        error: None,
                    }),
                    InteractionEvent::TreeConfig(config),
                ])
            }
            InteractionCommand::LoadTreeConfig(config) => {
                tree_support.import_tree_config(tree, config)?;
                let mut blackboards = vec![];
                let mut nodes = vec![];
                for blackboard_id in tree.blackboards() {
                    blackboards.push(Self::blackboard_information(
                        tree_support,
                        blackboard_id,
                        tree,
                    )?);
                }
                for node_id in tree.nodes() {
                    nodes.push(Self::node_information(tree_support, node_id, tree)?);
                }
                let roots = TreeRoots {
                    roots: tree.roots(),
                };
                let tree_state = TreeState {
                    blackboards,
                    nodes,
                    roots,
                };
                Ok(vec![
                    InteractionEvent::CommandResult(CommandResult {
                        command: self.clone(),
                        error: None,
                    }),
                    InteractionEvent::TreeState(tree_state),
                ])
            }
            InteractionCommand::Clear => {
                tree.clear()?;
                Ok(vec![InteractionEvent::CommandResult(CommandResult {
                    command: self.clone(),
                    error: None,
                })])
            }
            InteractionCommand::ResetNodes => {
                tree.reset_nodes();
                Ok(vec![InteractionEvent::CommandResult(CommandResult {
                    command: self.clone(),
                    error: None,
                })])
            }
            InteractionCommand::ResetNode(id) => {
                tree.reset_node(*id)?;
                Ok(vec![InteractionEvent::CommandResult(CommandResult {
                    command: self.clone(),
                    error: None,
                })])
            }
            InteractionCommand::SetBlackboardName(blackboard_id, name) => {
                tree.set_blackboard_name(*blackboard_id, name.as_deref())?;
                Ok(vec![
                    InteractionEvent::CommandResult(CommandResult {
                        command: self.clone(),
                        error: None,
                    }),
                    InteractionEvent::BlackboardInformation(Self::blackboard_information(
                        tree_support,
                        *blackboard_id,
                        tree,
                    )?),
                ])
            }
            InteractionCommand::SetDirectory(dir) => {
                let dir = dir.as_ref().map(|s| std::path::PathBuf::from(&s));
                let dir = dir.as_deref();
                tree.set_directory(dir);
                Ok(vec![InteractionEvent::CommandResult(CommandResult {
                    command: self.clone(),
                    error: None,
                })])
            }
            InteractionCommand::TreeCall(f) => {
                (*f).call(tree)?;
                Ok(vec![])
            } // e => Err(format!("unhandled command {e:?}").into()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BlackboardValues {
    pub blackboards: std::collections::BTreeMap<BlackboardId, SerializedBlackboardValues>,
}
impl BlackboardValues {
    pub fn from_tree(tree_support: &TreeSupport, tree: &dyn Tree) -> Result<Self, BetulaError> {
        let mut res = BlackboardValues::default();
        let blackboards = tree.blackboards();
        for blackboard_id in blackboards {
            let bb = tree
                .blackboard_ref(blackboard_id)
                .ok_or(format!("cannot find {blackboard_id:?}"))?;
            let bb = bb.borrow_mut();
            let port_values = tree_support.blackboard_value_serialize(&**bb)?;
            res.blackboards.insert(blackboard_id, port_values);
        }
        Ok(res)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct NodeStatus {
    pub node: NodeId,
    pub status: Result<ExecutionStatus, String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecutionResult {
    pub node_status: Vec<NodeStatus>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeInformation {
    pub id: NodeId,
    pub name: Option<String>,
    pub node_type: NodeType,
    pub config: Option<SerializedConfig>,
    pub children: Vec<NodeId>,
}

// pub type BlackboardMap
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlackboardInformation {
    pub id: BlackboardId,
    pub connections: Vec<PortConnection>,
    pub port_values: SerializedBlackboardValues,
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommandResult {
    pub command: InteractionCommand,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TreeState {
    pub nodes: Vec<NodeInformation>,
    pub blackboards: Vec<BlackboardInformation>,
    pub roots: TreeRoots,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TreeRoots {
    pub roots: Vec<NodeId>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum InteractionEvent {
    /// Result of a command, including error if failure.
    CommandResult(CommandResult),

    /// Information about a blackboard, its connections and states.
    BlackboardInformation(BlackboardInformation),

    /// Information about changed blackboard values.
    BlackboardValues(BlackboardValues),

    /// Execution results for nodes.
    ExecutionResult(ExecutionResult),

    /// Information about a node, its children and config.
    NodeInformation(NodeInformation),

    /// Current root nodes in the tree.
    TreeRoots(TreeRoots),

    /// The current tree config.
    TreeConfig(TreeConfig),

    /// The entire current tree state.
    TreeState(TreeState),
}

//------------------------------------------------------------------------
pub trait TreeClient {
    fn send_command(&self, command: InteractionCommand) -> Result<(), BetulaError>;
    fn get_event(&self) -> Result<Option<InteractionEvent>, BetulaError>;
}

pub trait TreeServer {
    fn get_command(&self) -> Result<Option<InteractionCommand>, BetulaError>;
    fn send_event(&self, event: InteractionEvent) -> Result<(), BetulaError>;
}

use std::sync::mpsc::{Receiver, Sender, TryRecvError};
pub struct InProcessControlServer {
    receiver: Receiver<InteractionCommand>,
    sender: Sender<InteractionEvent>,
}
impl TreeServer for InProcessControlServer {
    fn get_command(&self) -> Result<Option<InteractionCommand>, BetulaError> {
        match self.receiver.try_recv() {
            Ok(data) => Ok(Some(data)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err("control pipe disconnect".into()),
        }
    }
    fn send_event(&self, event: InteractionEvent) -> Result<(), BetulaError> {
        self.sender.send(event).map_err(|e| format!("{e:?}").into())
    }
}

pub struct InProcessControlClient {
    sender: Sender<InteractionCommand>,
    receiver: Receiver<InteractionEvent>,
}

impl TreeClient for InProcessControlClient {
    fn send_command(&self, command: InteractionCommand) -> Result<(), BetulaError> {
        self.sender
            .send(command)
            .map_err(|e| format!("{e:?}").into())
    }
    fn get_event(&self) -> Result<Option<InteractionEvent>, BetulaError> {
        match self.receiver.try_recv() {
            Ok(data) => Ok(Some(data)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err("control pipe disconnect".into()),
        }
    }
}

pub fn internal_server_client() -> (InProcessControlServer, InProcessControlClient) {
    let (command_sender, command_receiver) = std::sync::mpsc::channel();
    let (event_sender, event_receiver) = std::sync::mpsc::channel();
    (
        InProcessControlServer {
            sender: event_sender,
            receiver: command_receiver,
        },
        InProcessControlClient {
            sender: command_sender,
            receiver: event_receiver,
        },
    )
}
