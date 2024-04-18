use crate::prelude::*;
use crate::{Node, NodeError, NodeStatus, NodeType};

/// Node that always returns [`NodeStatus::Failure`].
///
/// Node may have one child, in which case it gets executed but its status
/// is ignored, [`NodeStatus::Failure`] is always returned.
#[derive(Debug, Copy, Clone, Default)]
pub struct FailureNode {}
impl Node for FailureNode {
    fn tick(&mut self, ctx: &dyn RunContext) -> Result<NodeStatus, NodeError> {
        if ctx.children() == 1 {
            let _ = ctx.run(0)?;
        } else if ctx.children() > 1 {
            return Err(format!("{:?} had more than one child", Self::static_type()).into());
        }

        Ok(NodeStatus::Failure)
    }
    fn static_type() -> NodeType
    where
        Self: Sized,
    {
        "failure".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}
