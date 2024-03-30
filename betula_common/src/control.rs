use betula_core::{
    blackboard::Chalkable, BetulaError, BlackboardId, NodeConfig, NodeId, NodeStatus, NodeType,
    PortConnection, PortName,
};

// we want asynchronous control & interaction with the tree.

// do we need all of this...? :/

pub struct AddNodeCommand {
    id: NodeId,
    node_type: NodeType,
}

pub struct Relation {
    parent: NodeId,
    position: usize,
    child: NodeId,
}

pub struct SetConfigCommand {
    id: NodeId,
    config: Box<dyn NodeConfig>,
}

pub struct ExecutionCommand {
    running: bool,
    one_shot: bool,
}

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

pub struct BlackboardValueEvent {
    id: BlackboardId,
    name: PortName,
    value: Box<dyn Chalkable>,
}

pub enum NodeExecutionResult {
    Success(NodeStatus),
    Error(String),
}

pub struct ExecutionResult {
    nodes: Vec<(NodeId, NodeExecutionResult)>,
}

pub struct NodeInformationEvent {
    id: NodeId,
    node_type: String,
    config: Option<Box<dyn NodeConfig>>,
    children: Vec<NodeId>,
}

pub struct FailedCommand {
    command: InteractionCommand,
    error: String,
}

pub enum InteractionEvent {
    FailedCommand(FailedCommand),
    BlackboardChange(BlackboardValueEvent),
    ExecutionResult(ExecutionResult),
    NodeInformation(NodeInformationEvent),
}

pub trait TreeControl {
    fn command(&self, command: InteractionCommand) -> Result<(), BetulaError>;
    fn event(&self) -> Result<Option<InteractionEvent>, BetulaError>;
}
