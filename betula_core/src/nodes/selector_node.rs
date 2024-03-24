use crate::prelude::*;
use crate::{Node, NodeError, NodeStatus};

#[derive(Debug, Copy, Clone)]
pub struct SelectorNode {}
impl Node for SelectorNode {
    fn tick(&mut self, ctx: &dyn RunContext) -> Result<NodeStatus, NodeError> {
        for id in 0..ctx.children() {
            match ctx.run(id)? {
                NodeStatus::Failure => {}
                other => return Ok(other),
            }
        }

        // Reached here, all children must've failed.
        Ok(NodeStatus::Failure)
    }
}
