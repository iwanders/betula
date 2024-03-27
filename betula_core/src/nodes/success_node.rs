use crate::prelude::*;
use crate::{Node, NodeError, NodeStatus, NodeType};

#[derive(Debug, Copy, Clone, Default)]
pub struct SuccessNode {}
impl Node for SuccessNode {
    fn tick(&mut self, _ctx: &dyn RunContext) -> Result<NodeStatus, NodeError> {
        Ok(NodeStatus::Success)
    }

    fn static_type() -> NodeType
    where
        Self: Sized,
    {
        "success".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}
