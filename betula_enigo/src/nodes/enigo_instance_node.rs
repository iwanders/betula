use betula_core::node_prelude::*;

use crate::{EnigoBlackboard, EnigoRunner};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EnigoInstanceNodeConfig {
    delay: u32,
}
impl IsNodeConfig for EnigoInstanceNodeConfig {}

#[derive(Debug, Default)]
pub struct EnigoInstanceNode {
    instance: Option<EnigoBlackboard>,
    output: Output<EnigoBlackboard>,
    config_changed: bool,
    pub config: EnigoInstanceNodeConfig,
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
        if self.config_changed {
            if let Some(v) = self.instance.as_ref() {
                v.set_delay(self.config.delay)?;
                self.config_changed = false;
            }
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

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        let previous = self.config.delay;
        let r = self.config.load_node_config(config);
        if previous != self.config.delay {
            self.config_changed = true;
        }
        r
    }
}

#[cfg(feature = "betula_egui")]
mod ui_support {
    use super::*;
    use betula_egui::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for EnigoInstanceNode {
        fn ui_title(&self) -> String {
            "enigo ⌨".to_owned()
        }

        fn ui_config(
            &mut self,
            ctx: &dyn UiNodeContext,
            ui: &mut egui::Ui,
            _scale: f32,
        ) -> UiConfigResponse {
            let _ = ctx;
            let mut ui_response = UiConfigResponse::UnChanged;
            let r = ui.add(
                egui::DragValue::new(&mut self.config.delay)
                    .clamp_range(1..=10000)
                    .suffix("ms")
                    .update_while_editing(false),
            );
            if r.changed() {
                ui_response = UiConfigResponse::Changed;
            }

            ui_response
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
