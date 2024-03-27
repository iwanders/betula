use crate::prelude::*;
use crate::{Node, NodeError, NodeStatus, NodeType};

#[derive(Debug, Copy, Clone)]
pub struct FailureNode {}
impl Node for FailureNode {
    fn tick(&mut self, _ctx: &dyn RunContext) -> Result<NodeStatus, NodeError> {
        Ok(NodeStatus::Failure)
    }
    fn node_type(&self) -> NodeType {
        "failure".into()
    }
}
