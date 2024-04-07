use betula_core::{
    BetulaError, BlackboardId, NodeId, NodeStatus, NodeType, PortConnection, PortName,
};

use crate::{
    tree_support::SerializedBlackboardValues, tree_support::SerializedConfig,
    tree_support::SerializedValue,
};

use serde::{Deserialize, Serialize};
// we want asynchronous control & interaction with the tree.

// do we need all of this...? :/

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
    pub fn new(f: TT) -> Box<dyn TreeCall> {
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum InteractionCommand {
    AddNode(AddNodeCommand),
    RemoveNode(NodeId),

    SetChildren(SetChildren),

    AddBlackboard(BlackboardId),

    SetConfig(SetConfigCommand),

    PortDisconnectConnect(PortChanges),

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

    pub fn remove_node(id: NodeId) -> Self {
        InteractionCommand::RemoveNode(id)
    }
    pub fn connect_port(port_connection: PortConnection) -> Self {
        Self::port_disconnect_connect(&vec![], &vec![port_connection])
    }
    pub fn disconnect_port(port_connection: PortConnection) -> Self {
        Self::port_disconnect_connect(&vec![port_connection], &vec![])
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

    pub fn set_children(parent: NodeId, children: Vec<NodeId>) -> Self {
        InteractionCommand::SetChildren(SetChildren { parent, children })
    }
    pub fn tree_call<
        F: Fn(&mut dyn Tree) -> Result<(), BetulaError> + std::marker::Send + Clone + 'static,
    >(
        f: F,
    ) -> Self {
        InteractionCommand::TreeCall(TreeCallWrapper::new(f))
    }

    pub fn set_config(id: NodeId, config: SerializedConfig) -> Self {
        InteractionCommand::SetConfig(SetConfigCommand { id, config })
    }

    fn node_information(
        tree_support: &TreeSupport,
        node_id: NodeId,
        tree: &mut dyn Tree,
    ) -> Result<NodeInformation, BetulaError> {
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
        })
    }

    fn blackboard_information(
        tree_support: &TreeSupport,
        blackboard_id: BlackboardId,
        tree: &mut dyn Tree,
    ) -> Result<BlackboardInformation, BetulaError> {
        let _ = tree_support;
        let bb = tree
            .blackboard_mut(blackboard_id)
            .ok_or(format!("cannot find {blackboard_id:?}"))?;
        let port_values = tree_support.blackboard_value_serialize(&*bb)?;
        let connections = tree.blackboard_connections(blackboard_id);
        Ok(BlackboardInformation {
            id: blackboard_id,
            port_values,
            connections,
        })
    }

    pub fn execute(
        &self,
        tree_support: &TreeSupport,
        tree: &mut dyn Tree,
    ) -> Result<Vec<InteractionEvent>, BetulaError> {
        match self {
            InteractionCommand::AddNode(ref v) => {
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
            InteractionCommand::SetChildren(ref v) => {
                let _modified = tree.set_children(v.parent, &v.children)?;
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
            InteractionCommand::AddBlackboard(v) => {
                let blackboard = tree_support
                    .create_blackboard()
                    .ok_or(format!("cannot create blackboard, no factory"))?;
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
            InteractionCommand::TreeCall(f) => {
                (*f).call(tree)?;
                Ok(vec![])
            } // e => Err(format!("unhandled command {e:?}").into()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlackboardValueEvent {
    pub id: BlackboardId,
    pub name: PortName,
    pub value: SerializedValue,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum NodeExecutionResult {
    Success(NodeStatus),
    Error(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecutionResult {
    pub nodes: Vec<(NodeId, NodeExecutionResult)>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeInformation {
    pub id: NodeId,
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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommandResult {
    pub command: InteractionCommand,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum InteractionEvent {
    CommandResult(CommandResult),
    BlackboardInformation(BlackboardInformation),
    // ExecutionResult(ExecutionResult),
    NodeInformation(NodeInformation),
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

pub struct InProcessControl {}
impl InProcessControl {
    pub fn new() -> (InProcessControlServer, InProcessControlClient) {
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
}
