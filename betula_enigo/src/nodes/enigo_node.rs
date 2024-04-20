use betula_core::node_prelude::*;

use crate::{EnigoBlackboard, EnigoRunner};

#[derive(Debug, Default)]
pub struct EnigoNode {
    is_created: bool,
    output: Output<EnigoBlackboard>,
}

impl EnigoNode {
    pub fn new() -> Self {
        EnigoNode::default()
    }
}

impl Node for EnigoNode {
    fn execute(&mut self, _ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        if !self.is_created {
            let v = EnigoRunner::new()?;
            self.output.set(EnigoBlackboard { interface: Some(v) })?;
            self.is_created = true;
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
}

#[cfg(feature = "betula_egui")]
mod ui_support {
    use super::*;
    use betula_egui::{UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for EnigoNode {
        fn ui_title(&self) -> String {
            "enigo ".to_owned()
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("provider".to_owned()),
                UiNodeCategory::Name("enigo".to_owned()),
            ]
        }
        // fn ui_child_range(&self) -> std::ops::Range<usize> {
        // 0..0 // todo without this we encounter an unreachable in the ui!
        // }
    }
}
