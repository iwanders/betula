use betula_core::{
    blackboard::Chalkable, BetulaError, BlackboardId, NodeConfig, NodeId, NodeStatus, NodeType,
    PortConnection, PortName,
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
pub struct Relation {
    pub parent: NodeId,
    pub position: usize,
    pub child: NodeId,
}

#[derive(Debug, Clone)]
pub struct SetConfigCommand {
    pub id: NodeId,
    pub config: Box<dyn NodeConfig>,
}

#[derive(Debug, Clone)]
pub struct ExecutionCommand {
    pub running: bool,
    pub one_shot: bool,
}

#[derive(Debug, Clone)]
pub enum InteractionCommand {
    AddNode(AddNodeCommand),
    AddBlackboard(BlackboardId),

    SetConfig(SetConfigCommand),

    AddConnection(PortConnection),
    RemoveConnection(PortConnection),

    AddRelation(Relation),
    RemoveRelation(Relation),

    ExecutionCommand(ExecutionCommand),

    RequestNodes,
}

use crate::tree_support::TreeSupport;
use betula_core::Tree;

impl InteractionCommand {
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
                    InteractionEvent::NodeInformation(NodeInformationEvent {
                        id: v.id,
                        node_type: v.node_type.clone(),
                        config: None,
                        children: vec![],
                    }),
                ])
            }
            e => Err(format!("unhandled command {e:?}").into()),
        }
    }
}

#[derive(Debug)]
pub struct BlackboardValueEvent {
    pub id: BlackboardId,
    pub name: PortName,
    pub value: Box<dyn Chalkable>,
}

#[derive(Debug)]
pub enum NodeExecutionResult {
    Success(NodeStatus),
    Error(String),
}

#[derive(Debug)]
pub struct ExecutionResult {
    pub nodes: Vec<(NodeId, NodeExecutionResult)>,
}

#[derive(Debug)]
pub struct NodeInformationEvent {
    pub id: NodeId,
    pub node_type: NodeType,
    pub config: Option<Box<dyn NodeConfig>>,
    pub children: Vec<NodeId>,
}

#[derive(Debug)]
pub struct CommandResult {
    pub command: InteractionCommand,
    pub error: Option<String>,
}

#[derive(Debug)]
pub enum InteractionEvent {
    CommandResult(CommandResult),
    BlackboardChange(BlackboardValueEvent),
    ExecutionResult(ExecutionResult),
    NodeInformation(NodeInformationEvent),
}

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