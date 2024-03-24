use crate::prelude::*;
use crate::{Node, NodeError, NodeStatus};

#[derive(Debug, Copy, Clone)]
pub struct SequenceNode {}
impl Node for SequenceNode {
    fn tick(&mut self, ctx: &dyn RunContext) -> Result<NodeStatus, NodeError> {
        for id in 0..ctx.children() {
            match ctx.run(id)? {
                NodeStatus::Success => {}
                other => return Ok(other), // fail or running.
            }
        }

        // All children succeeded.
        Ok(NodeStatus::Success)
    }
}
