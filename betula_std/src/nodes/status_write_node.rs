use betula_core::node_prelude::*;

/// Node that writes the child's execution status to a blackboard.
///
/// One output port `status`, of type [`ExecutionStatus`], which is the status of the child node.
#[derive(Debug, Default)]
pub struct StatusWriteNode {
    status_output: Output<ExecutionStatus>,
}

impl StatusWriteNode {
    pub fn new() -> Self {
        StatusWriteNode::default()
    }
}

impl Node for StatusWriteNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        if ctx.children() != 1 {
            return Err("RetryNode must have exactly one child node".into());
        }
        let res = ctx.run(0)?;
        self.status_output.set(res)?;
        Ok(res)
    }
    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![Port::output::<ExecutionStatus>("status")])
    }
    fn setup_outputs(
        &mut self,
        interface: &mut dyn BlackboardOutputInterface,
    ) -> Result<(), NodeError> {
        self.status_output =
            interface.output::<ExecutionStatus>("status", ExecutionStatus::Running)?;
        Ok(())
    }

    fn static_type() -> NodeType {
        "std_status_write".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}

#[cfg(feature = "betula_editor")]
pub mod ui_support {
    use super::*;
    use betula_editor::{UiNode, UiNodeCategory};

    impl UiNode for StatusWriteNode {
        fn ui_title(&self) -> String {
            "status 🖊".to_owned()
        }

        fn ui_child_range(&self) -> std::ops::Range<usize> {
            1..1
        }
        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("provider".to_owned()),
                UiNodeCategory::Name("status".to_owned()),
            ]
        }
    }
}
