use betula_core::node_prelude::*;

use crate::{EnigoBlackboard, EnigoRunner};

#[derive(Debug, Default)]
pub struct EnigoInstanceNode {
    instance: Option<EnigoBlackboard>,
    output: Output<EnigoBlackboard>,
}

impl EnigoInstanceNode {
    pub fn new() -> Self {
        EnigoInstanceNode::default()
    }
}

impl Node for EnigoInstanceNode {
    fn execute(&mut self, _ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        if self.instance.is_none() {
            let v = EnigoRunner::new()?;
            let instance = EnigoBlackboard { interface: Some(v) };
            self.instance = Some(instance.clone());
            self.output.set(instance)?;
        }
        Ok(ExecutionStatus::Success)
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![Port::output::<EnigoBlackboard>("enigo")])
    }
    fn setup_outputs(
        &mut self,
        interface: &mut dyn BlackboardOutputInterface,
    ) -> Result<(), NodeError> {
        self.output = interface.output::<EnigoBlackboard>("enigo", EnigoBlackboard::default())?;
        Ok(())
    }

    fn static_type() -> NodeType {
        "enigo_provider".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }

    fn reset(&mut self) {
        self.instance = None;
    }
}

#[cfg(feature = "betula_editor")]
mod ui_support {
    use super::*;
    use betula_editor::{UiNode, UiNodeCategory};

    impl UiNode for EnigoInstanceNode {
        fn ui_title(&self) -> String {
            "enigo ⌨".to_owned()
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("provider".to_owned()),
                UiNodeCategory::Name("enigo".to_owned()),
            ]
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..0
        }
    }
}
