use betula_core::prelude::*;
use betula_core::{ExecutionStatus, Node, NodeError, NodeType};

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

#[cfg(feature = "betula_editor")]
pub mod ui_support {
    use super::*;
    use betula_editor::{UiNode, UiNodeCategory};

    impl UiNode for SuccessNode {
        fn ui_title(&self) -> String {
            "success ✔".to_owned()
        }
        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("decorator".to_owned()),
                UiNodeCategory::Group("core".to_owned()),
                UiNodeCategory::Name("success".to_owned()),
            ]
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..1
        }
    }
}
