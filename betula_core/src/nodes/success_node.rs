use crate::prelude::*;
use crate::{Node, NodeError, NodeStatus};

#[derive(Debug, Copy, Clone)]
pub struct SuccessNode {}
impl Node for SuccessNode {
    fn tick(&mut self, _ctx: &dyn RunContext) -> Result<NodeStatus, NodeError> {
        Ok(NodeStatus::Success)
    }
}
