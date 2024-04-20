use crate::prelude::*;
use crate::{ExecutionStatus, Node, NodeError, NodeType};

/// Node that always returns [`ExecutionStatus::Success`].
///
/// Node may have one child, in which case it gets executed but its status
/// is ignored, [`ExecutionStatus::Success`] is always returned.
#[derive(Debug, Copy, Clone, Default)]
pub struct SuccessNode {}
impl Node for SuccessNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        if ctx.children() == 1 {
            let _ = ctx.run(0)?;
        } else if ctx.children() > 1 {
            return Err(format!("{:?} had more than one child", Self::static_type()).into());
        }

        Ok(ExecutionStatus::Success)
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
