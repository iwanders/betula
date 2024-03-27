use crate::prelude::*;
use crate::{Node, NodeError, NodeStatus, NodeType};

#[derive(Debug, Copy, Clone)]
pub struct SuccessNode {}
impl Node for SuccessNode {
    fn tick(&mut self, _ctx: &dyn RunContext) -> Result<NodeStatus, NodeError> {
        Ok(NodeStatus::Success)
    }

    fn node_type(&self) -> NodeType {
        "success".into()
    }
}
