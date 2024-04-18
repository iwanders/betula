use crate::prelude::*;
use crate::{Node, NodeError, NodeStatus, NodeType};

/// Node that executes nodes in sequence returning the first non-[`NodeStatus::Failure`].
///
/// Runs nodes from left to right, ignoring [`NodeStatus::Failure`] but
/// returning the first [`NodeStatus::Success`] or [`NodeStatus::Running`]
/// encountered, at this point that value is returned.
/// The node returns [`NodeStatus::Failure`] if all child nodes failed.
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
