use crate::prelude::*;
use crate::{Node, NodeError, NodeStatus, NodeType};

/// Node that executes nodes in sequence until one does not return [`NodeStatus::Success`].
///
/// Runs nodes from left to right, ignoring [`NodeStatus::Success`] but
/// returning the first [`NodeStatus::Failure`] or [`NodeStatus::Running`]
/// encountered, at this point that value is returned.
/// The node returns [`NodeStatus::Success`] if all child nodes succceed.
#[derive(Debug, Copy, Clone, Default)]
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

    fn static_type() -> NodeType
    where
        Self: Sized,
    {
        "sequence".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}
