use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

use crate::OverlaySupport;
// use std::sync::Arc;

use crate::OverlayBlackboard;
use egui::Color32;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OverlayTextNodeConfig {
    #[serde(default)]
    pub position: (i32, i32),

    #[serde(default)]
    pub size: (u32, u32),

    #[serde(default)]
    pub font_size: f32,

    #[serde(default)]
    pub text_color: Color32,

    #[serde(default)]
    pub fill_color: Color32,
}
impl IsNodeConfig for OverlayTextNodeConfig {}

impl Default for OverlayTextNodeConfig {
    fn default() -> Self {
        OverlayTextNodeConfig {
            position: (0, 0),
            size: (100, 100),
            // draw_border: true,
            font_size: 64.0,
            text_color: Color32::BLACK,
            fill_color: Color32::TRANSPARENT,
        }
    }
}
impl OverlayTextNodeConfig {
    #[cfg(feature = "use_client_server")]
    pub fn to_instruction(&self, text: &str) -> crate::client_server::instructions::Text {
        crate::client_server::instructions::Text {
            position: (self.position.0 as f32, self.position.1 as f32),
            size: (self.size.0 as f32, self.size.1 as f32),
            font_size: self.font_size,
            text_color: self.text_color,
            fill_color: self.fill_color,
            text: text.to_owned(),
            ..Default::default()
        }
    }
}

#[derive(Debug)]
struct CurrentLabel {
    text: String,
    text_token: screen_overlay::VisualId,
}

#[derive(Debug, Default)]
pub struct OverlayTextNode {
    input_instance: Input<OverlayBlackboard>,
    input_text: Input<String>,

    text_label: Option<CurrentLabel>,

    token_to_remove: Option<screen_overlay::VisualId>,

    needs_update: bool,

    pub config: OverlayTextNodeConfig,
}

impl OverlayTextNode {
    pub fn new() -> Self {
        OverlayTextNode::default()
    }
}

impl Node for OverlayTextNode {
    fn execute(&mut self, _ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        let interface = self.input_instance.get()?;
        let interface = interface
            .interface
            .as_ref()
            .map(|v| &v.overlay)
            .ok_or("missing interface")?;
        let desired_text = self.input_text.get()?;
        let desired_text_lambda = desired_text.clone();
        let needs_update = self
            .text_label
            .as_ref()
            .map(|a| a.text != desired_text)
            .unwrap_or(true)
            || self.needs_update;

        if needs_update {
            self.needs_update = false;
            let id = if let Some(old_token) = self.token_to_remove.take() {
                interface.remove_and_set_text(&old_token, &self.config, &desired_text_lambda)?
            } else {
                if let Some(old_token) = self.text_label.take().map(|z| z.text_token) {
                    interface.remove_and_set_text(&old_token, &self.config, &desired_text_lambda)?
                } else {
                    interface.set_text(&self.config, &desired_text_lambda)?
                }
            };

            self.text_label = Some(CurrentLabel {
                text: desired_text,
                text_token: id,
            });
        }
        Ok(ExecutionStatus::Success)
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![
            Port::input::<OverlayBlackboard>("overlay"),
            Port::input::<String>("text"),
        ])
    }
    fn setup_inputs(
        &mut self,
        interface: &mut dyn BlackboardInputInterface,
    ) -> Result<(), NodeError> {
        self.input_instance = interface.input::<OverlayBlackboard>("overlay")?;
        self.input_text = interface.input::<String>("text")?;
        self.needs_update = true;
        Ok(())
    }

    fn static_type() -> NodeType {
        "overlay_text".into()
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        let r = self.config.load_node_config(config);
        self.reset();
        r
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }

    fn reset(&mut self) {
        if let Some(old_token) = self.text_label.take() {
            self.token_to_remove = Some(old_token.text_token);
        }
        self.needs_update = true;
    }
}

#[cfg(feature = "betula_editor")]
mod ui_support {
    use super::*;
    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for OverlayTextNode {
        fn ui_title(&self) -> String {
            "text 🗛".to_owned()
        }

        fn ui_config(
            &mut self,
            ctx: &dyn UiNodeContext,
            ui: &mut egui::Ui,
            _scale: f32,
        ) -> UiConfigResponse {
            let _ = ctx;
            let mut modified = false;
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("pos: ");
                    modified |= ui
                        .add(egui::DragValue::new(&mut self.config.position.0).range(0..=10000))
                        .changed();
                    modified |= ui
                        .add(egui::DragValue::new(&mut self.config.position.1).range(0..=10000))
                        .changed();
                });
                ui.horizontal(|ui| {
                    ui.label("size: ");
                    modified |= ui
                        .add(egui::DragValue::new(&mut self.config.size.0).range(1..=10000))
                        .changed();
                    modified |= ui
                        .add(egui::DragValue::new(&mut self.config.size.1).range(1..=10000))
                        .changed();
                });
                ui.horizontal(|ui| {
                    // modified |= ui
                    //     .add(egui::Checkbox::new(&mut self.config.draw_border, "Border?"))
                    //     .changed();
                    ui.label("size: ");
                    modified |= ui
                        .add(egui::DragValue::new(&mut self.config.font_size).range(0.0..=10000.0))
                        .changed();
                });
                ui.horizontal(|ui| {
                    ui.label("color: ");
                    modified |= ui
                        .color_edit_button_srgba(&mut self.config.text_color)
                        .changed();
                    ui.label("fill: ");
                    modified |= ui
                        .color_edit_button_srgba(&mut self.config.fill_color)
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
                UiNodeCategory::Folder("consumer".to_owned()),
                UiNodeCategory::Name("overlay_text".to_owned()),
            ]
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..0
        }
    }
}
