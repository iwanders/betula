use betula_core::node_prelude::*;

use crate::{HotkeyBlackboard, HotkeyInterface};

#[derive(Debug, Default)]
pub struct HotkeyInstanceNode {
    instance: Option<HotkeyBlackboard>,
    output: Output<HotkeyBlackboard>,
}

impl HotkeyInstanceNode {
    pub fn new() -> Self {
        HotkeyInstanceNode::default()
    }
}

impl Node for HotkeyInstanceNode {
    fn execute(&mut self, _ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        if self.instance.is_none() {
            let v = HotkeyInterface::new()?;
            let instance = HotkeyBlackboard { interface: Some(v) };
            self.instance = Some(instance.clone());
            self.output.set(instance)?;
        }
        Ok(ExecutionStatus::Success)
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![Port::output::<HotkeyBlackboard>("hotkey")])
    }
    fn setup_outputs(
        &mut self,
        interface: &mut dyn BlackboardOutputInterface,
    ) -> Result<(), NodeError> {
        self.output =
            interface.output::<HotkeyBlackboard>("hotkey", HotkeyBlackboard::default())?;
        Ok(())
    }

    fn static_type() -> NodeType {
        "hotkey_provider".into()
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

    impl UiNode for HotkeyInstanceNode {
        fn ui_title(&self) -> String {
            "hotkey 🗝".to_owned()
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("provider".to_owned()),
                UiNodeCategory::Name("hotkey".to_owned()),
            ]
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..0
        }
    }
}
