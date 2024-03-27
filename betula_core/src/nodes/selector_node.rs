use crate::prelude::*;
use crate::{Node, NodeError, NodeStatus, NodeType};

#[derive(Debug, Copy, Clone, Default)]
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

    fn static_type() -> NodeType
    where
        Self: Sized,
    {
        "selector".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}
