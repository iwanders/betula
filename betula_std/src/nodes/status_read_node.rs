use betula_core::node_prelude::*;

/// Node that reads the status and returns that, can be used as a decorator.
///
/// One input port `status`, of type [`ExecutionStatus`], which is the returned value regardless of
/// what the optional child node returns.
#[derive(Debug, Default)]
pub struct StatusReadNode {
    status_input: Input<ExecutionStatus>,
}

impl StatusReadNode {
    pub fn new() -> Self {
        StatusReadNode::default()
    }
}

impl Node for StatusReadNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        let status = self.status_input.get()?;
        if ctx.children() == 1 {
            let _ = ctx.run(0)?;
        } else if ctx.children() > 1 {
            return Err("status read node may only have up to one child".into());
        }

        Ok(status)
    }
    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![Port::input::<ExecutionStatus>("status")])
    }
    fn setup_inputs(
        &mut self,
        interface: &mut dyn BlackboardInputInterface,
    ) -> Result<(), NodeError> {
        self.status_input = interface.input::<ExecutionStatus>("status")?;
        Ok(())
    }

    fn static_type() -> NodeType {
        "std_status_read".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}

#[cfg(feature = "betula_editor")]
pub mod ui_support {
    use super::*;
    use betula_editor::{UiNode, UiNodeCategory};

    impl UiNode for StatusReadNode {
        fn ui_title(&self) -> String {
            "status 👓".to_owned()
        }

        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..1
        }
        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("conditional".to_owned()),
                UiNodeCategory::Name("status".to_owned()),
            ]
        }
    }
}
