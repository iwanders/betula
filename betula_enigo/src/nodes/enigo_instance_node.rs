use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

use crate::{EnigoBlackboard, EnigoRunner};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EnigoInstanceNodeConfig {
    #[serde(default)]
    pub windows_offset: (i32, i32),
    #[serde(default)]
    pub linux_offset: (i32, i32),
}
impl IsNodeConfig for EnigoInstanceNodeConfig {}

#[derive(Debug, Default)]
pub struct EnigoInstanceNode {
    instance: Option<EnigoBlackboard>,
    output: Output<EnigoBlackboard>,
    pub config: EnigoInstanceNodeConfig,
}

impl EnigoInstanceNode {
    pub fn new() -> Self {
        EnigoInstanceNode::default()
    }
    fn cursor_offset(&self) -> (i32, i32) {
        const IS_WINDOWS: bool = cfg!(target_os = "windows");
        if IS_WINDOWS {
            self.config.windows_offset
        } else {
            self.config.linux_offset
        }
    }
}

impl Node for EnigoInstanceNode {
    fn execute(&mut self, _ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        if self.instance.is_none() {
            let v = EnigoRunner::new()?;
            let instance = EnigoBlackboard { interface: Some(v) };
            instance.set_cursor_offset(self.cursor_offset())?;
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

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        let r = self.config.load_node_config(config);
        if let Some(instance) = &self.instance {
            instance.set_cursor_offset(self.cursor_offset())?;
        }
        r
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
    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

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
            // let mut ui_response = UiConfigResponse::UnChanged;
            let mut modified = false;
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("linux Δ: ");
                    modified |= ui
                        .add(egui::DragValue::new(&mut self.config.linux_offset.0))
                        .changed();
                    modified |= ui
                        .add(egui::DragValue::new(&mut self.config.linux_offset.1))
                        .changed();
                });
                ui.horizontal(|ui| {
                    ui.label("windows Δ: ");
                    modified |= ui
                        .add(egui::DragValue::new(&mut self.config.windows_offset.0))
                        .changed();
                    modified |= ui
                        .add(egui::DragValue::new(&mut self.config.windows_offset.1))
                        .changed();
                });
            });

            if modified {
                UiConfigResponse::Changed
            } else {
                UiConfigResponse::UnChanged
            }
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
