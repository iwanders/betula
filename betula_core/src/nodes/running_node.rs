use crate::prelude::*;
use crate::{Node, NodeError, NodeStatus, NodeType};

#[derive(Debug, Copy, Clone, Default)]
pub struct RunningNode {}
impl Node for RunningNode {
    fn tick(&mut self, ctx: &dyn RunContext) -> Result<NodeStatus, NodeError> {
        if ctx.children() == 1 {
            let _ = ctx.run(0)?;
        } else if ctx.children() > 1 {
            return Err(format!("{:?} had more than one child", Self::static_type()).into());
        }

        Ok(NodeStatus::Running)
    }

    fn static_type() -> NodeType
    where
        Self: Sized,
    {
        "running".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}
