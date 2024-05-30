use betula_core::node_prelude::*;

/// Node that negates the execution status.
///
/// Node must have one child, if it returns [`ExecutionStatus::Success`] it is turned into
/// [`ExecutionStatus::Failure`], and [`ExecutionStatus::Failure`] is turned
/// into [`ExecutionStatus::Success`], [`ExecutionStatus::Running`] is passed unchanged.
#[derive(Debug, Copy, Clone, Default)]
pub struct NegateNode {}
impl Node for NegateNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        if ctx.children() != 1 {
            return Err(format!("{:?} must have exactly one child", Self::static_type()).into());
        }

        match ctx.run(0)? {
            ExecutionStatus::Success => Ok(ExecutionStatus::Failure),
            ExecutionStatus::Failure => Ok(ExecutionStatus::Success),
            ExecutionStatus::Running => Ok(ExecutionStatus::Running),
        }
    }

    fn static_type() -> NodeType
    where
        Self: Sized,
    {
        "std_negate".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}

#[cfg(feature = "betula_editor")]
pub mod ui_support {
    use super::*;
    use betula_editor::{UiNode, UiNodeCategory};

    impl UiNode for NegateNode {
        fn ui_title(&self) -> String {
            "negate ！".to_owned()
        }
        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("decorator".to_owned()),
                UiNodeCategory::Group("core".to_owned()),
                UiNodeCategory::Name("negate".to_owned()),
            ]
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            1..1
        }
    }
}
