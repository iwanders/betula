use crate::prelude::*;
use crate::{ExecutionStatus, Node, NodeError, NodeType};

/// Node that executes nodes in sequence until one does not return [`ExecutionStatus::Success`].
///
/// Runs nodes from left to right, ignoring [`ExecutionStatus::Success`] but
/// returning the first [`ExecutionStatus::Failure`] or [`ExecutionStatus::Running`]
/// encountered, at this point that value is returned.
/// The node returns [`ExecutionStatus::Success`] if all child nodes succceed.
#[derive(Debug, Copy, Clone, Default)]
pub struct SequenceNode {}
impl Node for SequenceNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        for id in 0..ctx.children() {
            match ctx.run(id)? {
                ExecutionStatus::Success => {}
                other => return Ok(other), // fail or running.
            }
        }

        // All children succeeded.
        Ok(ExecutionStatus::Success)
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
