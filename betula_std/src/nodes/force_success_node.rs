use betula_core::node_prelude::*;

/// Node that changes the non-running status to the desired value.
///
/// Node must have one child, if it returns [`ExecutionStatus::Running`] unchanged, but both
/// [`ExecutionStatus::Success`] and [`ExecutionStatus::Failure`] are turned into
/// [`ExecutionStatus::Success`].
#[derive(Debug, Copy, Clone, Default)]
pub struct ForceSuccessNode {}
impl Node for ForceSuccessNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        if ctx.children() != 1 {
            return Err(format!("{:?} must have exactly one child", Self::static_type()).into());
        }

        match ctx.run(0)? {
            ExecutionStatus::Success => Ok(ExecutionStatus::Success),
            ExecutionStatus::Failure => Ok(ExecutionStatus::Success),
            ExecutionStatus::Running => Ok(ExecutionStatus::Running),
        }
    }

    fn static_type() -> NodeType
    where
        Self: Sized,
    {
        "std_force_success".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}

#[cfg(feature = "betula_editor")]
pub mod ui_support {
    use super::*;
    use betula_editor::{UiNode, UiNodeCategory};

    impl UiNode for ForceSuccessNode {
        fn ui_title(&self) -> String {
            "force success ✔".to_owned()
        }
        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("decorator".to_owned()),
                UiNodeCategory::Group("core".to_owned()),
                UiNodeCategory::Name("force_success".to_owned()),
            ]
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            1..1
        }
    }
}
