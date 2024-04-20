use crate::prelude::*;
use crate::{ExecutionStatus, Node, NodeError, NodeType};

/// Node that executes nodes in sequence returning the first non-[`ExecutionStatus::Failure`].
///
/// Runs nodes from left to right, ignoring [`ExecutionStatus::Failure`] but
/// returning the first [`ExecutionStatus::Success`] or [`ExecutionStatus::Running`]
/// encountered, at this point that value is returned.
/// The node returns [`ExecutionStatus::Failure`] if all child nodes failed.
#[derive(Debug, Copy, Clone, Default)]
pub struct SelectorNode {}
impl Node for SelectorNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        for id in 0..ctx.children() {
            match ctx.run(id)? {
                ExecutionStatus::Failure => {}
                other => return Ok(other),
            }
        }

        // Reached here, all children must've failed.
        Ok(ExecutionStatus::Failure)
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
