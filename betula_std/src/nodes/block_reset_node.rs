use betula_core::prelude::*;
use betula_core::{ExecutionStatus, Node, NodeError, NodeType};

/// Node that blocks the propagation of node resets.
///
/// This node blocks the propagation of the [`Node::reset_children`] recursive reset behaviour.
/// This means that any nodes below this one will not be reset. This can be helpful if one
/// part of the tree is reachable through two paths, but should only be reset by one of them.
#[derive(Debug, Copy, Clone, Default)]
pub struct BlockResetNode {}
impl Node for BlockResetNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        if ctx.children() != 1 {
            return Err(format!("{:?} should have exactly one child", Self::static_type()).into());
        }
        ctx.run(0)
    }

    fn static_type() -> NodeType {
        "std_block_reset".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }

    fn reset_recursive(&mut self, ctx: &dyn ResetContext) -> Result<(), NodeError> {
        let _ = ctx;
        // self.reset();
        // for z in 0..ctx.children() {
        // ctx.reset_recursive(z)?;
        // }
        // Recursion stops here.
        Ok(())
    }
}

#[cfg(feature = "betula_editor")]
pub mod ui_support {
    use super::*;
    use betula_editor::{UiNode, UiNodeCategory};

    impl UiNode for BlockResetNode {
        fn ui_title(&self) -> String {
            "block reset ⬣".to_owned()
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("decorator".to_owned()),
                UiNodeCategory::Group("core".to_owned()),
                UiNodeCategory::Name("block_reset".to_owned()),
            ]
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            1..1
        }
    }
}
