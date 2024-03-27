use crate::prelude::*;
use crate::{Node, NodeError, NodeStatus, NodeType};

#[derive(Debug, Copy, Clone, Default)]
pub struct FailureNode {}
impl Node for FailureNode {
    fn tick(&mut self, _ctx: &dyn RunContext) -> Result<NodeStatus, NodeError> {
        Ok(NodeStatus::Failure)
    }
    fn static_type() -> NodeType
    where
        Self: Sized,
    {
        "failure".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}
