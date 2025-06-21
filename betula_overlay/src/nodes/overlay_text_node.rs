use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

// use std::sync::Arc;

use crate::OverlayBlackboard;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OverlayTextNodeConfig {
    #[serde(default)]
    pub position: (i32, i32),

    #[serde(default)]
    pub size: (u32, u32),

    #[serde(default)]
    pub draw_border: bool,

    #[serde(default)]
    pub font_size: f32,

    #[serde(default)]
    pub text_color: screen_overlay::Color,
}
impl IsNodeConfig for OverlayTextNodeConfig {}

impl Default for OverlayTextNodeConfig {
    fn default() -> Self {
        OverlayTextNodeConfig {
            position: (0, 0),
            size: (100, 100),
            draw_border: true,
            font_size: 64.0,
            text_color: Default::default(),
        }
    }
}

#[derive(Debug)]
struct CurrentLabel {
    text: String,
    _text_token: screen_overlay::VisualToken,
    _border_token: Option<screen_overlay::VisualToken>,
}

#[derive(Debug, Default)]
pub struct OverlayTextNode {
    input_instance: Input<OverlayBlackboard>,
    input_text: Input<String>,

    text_label: Option<CurrentLabel>,

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
        let needs_update = self
            .text_label
            .as_ref()
            .map(|a| a.text != desired_text)
            .unwrap_or(true)
            || self.needs_update;

        if needs_update {
            self.needs_update = false;
            use screen_overlay::{
                Color, DashStyle, DrawGeometry, LineStyle, Rect, Stroke, TextAlignment,
                TextProperties,
            };
            let bounding_box =
                Rect::from(self.config.position.0 as f32, self.config.position.1 as f32)
                    .sized(self.config.size.0 as f32, self.config.size.1 as f32);
            let geometry = DrawGeometry::new().rectangle(&bounding_box);

            let border_token = if self.config.draw_border {
                let color = Color {
                    r: 255,
                    g: 0,
                    b: 255,
                    a: 255,
                };
                let stroke = Stroke { color, width: 1.0 };
                let text_box_style = LineStyle {
                    dash_style: DashStyle::Dash,
                    // line_join: LineJoin::Round,
                    ..Default::default()
                };

                Some(interface.draw_geometry(&geometry, &stroke, &text_box_style)?)
            } else {
                None
            };

            let font = interface.prepare_font(&TextProperties {
                size: self.config.font_size,
                horizontal_align: TextAlignment::Min,
                vertical_align: TextAlignment::Min,
                ..Default::default()
            })?;

            let text_token = interface
                .draw_text(&desired_text, &bounding_box, &self.config.text_color, &font)
                .expect("create image failed");
            self.text_label = Some(CurrentLabel {
                text: desired_text,
                _text_token: text_token,
                _border_token: border_token,
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
        self.text_label.take();
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
            // let mut ui_response = UiConfigResponse::UnChanged;
            /*
                position: (0, 0),
                size: (100, 100),
                draw_border: true,
                font_size: 32.0,
                text_color: Default::default(),
            */
            let mut modified = false;
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("pos: ");
                    modified |= ui
                        .add(
                            egui::DragValue::new(&mut self.config.position.0)
                                .clamp_range(0..=10000),
                        )
                        .changed();
                    modified |= ui
                        .add(
                            egui::DragValue::new(&mut self.config.position.1)
                                .clamp_range(0..=10000),
                        )
                        .changed();
                });
                ui.horizontal(|ui| {
                    ui.label("size: ");
                    modified |= ui
                        .add(egui::DragValue::new(&mut self.config.size.0).clamp_range(1..=10000))
                        .changed();
                    modified |= ui
                        .add(egui::DragValue::new(&mut self.config.size.1).clamp_range(1..=10000))
                        .changed();
                });
                ui.horizontal(|ui| {
                    modified |= ui
                        .add(egui::Checkbox::new(&mut self.config.draw_border, "Border?"))
                        .changed();
                    ui.label("size: ");
                    modified |= ui
                        .add(
                            egui::DragValue::new(&mut self.config.font_size)
                                .clamp_range(0.0..=10000.0),
                        )
                        .changed();
                });
                ui.horizontal(|ui| {
                    ui.label("color: ");
                    let mut rgba = [
                        self.config.text_color.r as f32 / 255.0,
                        self.config.text_color.g as f32 / 255.0,
                        self.config.text_color.b as f32 / 255.0,
                        self.config.text_color.a as f32 / 255.0,
                    ];
                    let color_changed = ui.color_edit_button_rgba_unmultiplied(&mut rgba).changed();
                    if color_changed {
                        self.config.text_color.r = (rgba[0] * 255.0) as u8;
                        self.config.text_color.g = (rgba[1] * 255.0) as u8;
                        self.config.text_color.b = (rgba[2] * 255.0) as u8;
                        self.config.text_color.a = (rgba[3] * 255.0) as u8;
                    }
                    modified |= color_changed;
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
