use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

use std::sync::Arc;

use crate::{OverlayBlackboard, OverlayInterface};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OverlayInstanceNodeConfig {
    #[serde(default)]
    pub windows_offset: (i32, i32),
    #[serde(default)]
    pub linux_offset: (i32, i32),
}
impl IsNodeConfig for OverlayInstanceNodeConfig {}

#[derive(Debug, Default)]
pub struct OverlayInstanceNode {
    instance: Option<OverlayInterface>,
    output: Output<OverlayBlackboard>,
    pub config: OverlayInstanceNodeConfig,
}

impl OverlayInstanceNode {
    pub fn new() -> Self {
        OverlayInstanceNode::default()
    }
}

impl Node for OverlayInstanceNode {
    fn execute(&mut self, _ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        if self.instance.is_none() {
            let instance = OverlayInterface::new()?;
            let value = OverlayBlackboard {
                interface: Some(instance.clone()),
            };
            // instance.set_cursor_offset(self.cursor_offset())?;
            self.instance = Some(instance.clone());
            self.output.set(value)?;
        }
        Ok(ExecutionStatus::Success)
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![Port::output::<OverlayBlackboard>("overlay")])
    }
    fn setup_outputs(
        &mut self,
        interface: &mut dyn BlackboardOutputInterface,
    ) -> Result<(), NodeError> {
        self.output =
            interface.output::<OverlayBlackboard>("overlay", OverlayBlackboard::default())?;
        Ok(())
    }

    fn static_type() -> NodeType {
        "overlay_provider".into()
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        let r = self.config.load_node_config(config);
        if let Some(instance) = &self.instance {
            // instance.set_cursor_offset(self.cursor_offset())?;
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

    impl UiNode for OverlayInstanceNode {
        fn ui_title(&self) -> String {
            "overlay ⌨".to_owned()
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
                UiNodeCategory::Name("overlay".to_owned()),
            ]
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..0
        }
    }
}
