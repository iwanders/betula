use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

// use std::sync::Arc;

use crate::{OverlayBlackboard, OverlayInterface};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OverlayInstanceNodeConfig {
    #[serde(default)]
    pub windows_config: screen_overlay::OverlayConfig,
    #[serde(default)]
    pub linux_config: screen_overlay::OverlayConfig,
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
            let config = if cfg!(target_os = "linux") {
                self.config.linux_config
            } else {
                self.config.windows_config
            };

            // This is a bit tricky, becaus eat creation we run into https://github.com/emilk/egui/issues/3632#issuecomment-3733528750

            let new_instance = OverlayInterface::new(config)?;
            self.instance = Some(new_instance);
        }
        if let Some(instance) = self.instance.as_ref() {
            let value = OverlayBlackboard {
                interface: Some(instance.clone()),
            };
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
        self.config.load_node_config(config)
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }

    fn reset(&mut self) {}
}

#[cfg(feature = "betula_editor")]
mod ui_support {
    use super::*;
    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for OverlayInstanceNode {
        fn ui_title(&self) -> String {
            "overlay 🎞".to_owned()
        }

        fn ui_config(
            &mut self,
            ctx: &dyn UiNodeContext,
            ui: &mut egui::Ui,
            _scale: f32,
        ) -> UiConfigResponse {
            // let _ = (ctx, ui);
            fn add_config_drawable(
                ui: &mut egui::Ui,
                config: &mut screen_overlay::OverlayConfig,
            ) -> bool {
                let mut modified = false;

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("pos");
                        modified |= ui
                            .add(egui::DragValue::new(&mut config.position[0]))
                            .changed();
                        modified |= ui
                            .add(egui::DragValue::new(&mut config.position[1]))
                            .changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("size");
                        modified |= ui.add(egui::DragValue::new(&mut config.size[0])).changed();
                        modified |= ui.add(egui::DragValue::new(&mut config.size[1])).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("bg: ");
                        modified |= ui
                            .color_edit_button_srgba(&mut config.central_panel_fill)
                            .changed();
                    });
                });
                modified
            };
            let mut modified = false;
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("linux: ");
                    modified |= add_config_drawable(ui, &mut self.config.linux_config);
                });
                ui.horizontal(|ui| {
                    ui.label("windows: ");
                    modified |= add_config_drawable(ui, &mut self.config.windows_config);
                });
            });

            if modified {
                UiConfigResponse::Changed
            } else {
                UiConfigResponse::UnChanged
            }
            // UiConfigResponse::UnChanged
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
