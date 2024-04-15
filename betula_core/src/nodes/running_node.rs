use crate::prelude::*;
use crate::{Node, NodeError, NodeStatus, NodeType};

#[derive(Debug, Copy, Clone, Default)]
pub struct RunningNode {}
impl Node for RunningNode {
    fn tick(&mut self, _ctx: &dyn RunContext) -> Result<NodeStatus, NodeError> {
        Ok(NodeStatus::Running)
    }

    fn static_type() -> NodeType
    where
        Self: Sized,
    {
        "running".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}
