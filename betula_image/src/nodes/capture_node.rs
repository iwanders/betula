use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

use crate::Image;
use screen_capture::{CaptureConfig, CaptureSpecification, ThreadedCapturer};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CaptureNodeConfig {
    capture: CaptureConfig,
}
impl IsNodeConfig for CaptureNodeConfig {}

#[derive(Default)]
pub struct CaptureNode {
    output: Output<Image>,
    output_time: Output<f64>,
    output_duration: Output<f64>,
    capture: Option<ThreadedCapturer>,
    config: CaptureNodeConfig,
}
impl std::fmt::Debug for CaptureNode {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "CaptureNode")
    }
}

impl CaptureNode {
    pub fn new() -> Self {
        CaptureNode::default()
    }
}

impl Node for CaptureNode {
    fn execute(&mut self, _ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        let c = self
            .capture
            .get_or_insert_with(|| ThreadedCapturer::new(self.config.capture.clone()));
        let info = c.latest();
        match info.result {
            Ok(img) => {
                use std::time::UNIX_EPOCH;
                self.output.set(Image::new(img))?;
                let _ = self
                    .output_time
                    .set(info.time.duration_since(UNIX_EPOCH)?.as_secs_f64());
                let _ = self.output_duration.set(info.duration.as_secs_f64());
                Ok(ExecutionStatus::Success)
            }
            Err(()) => Ok(ExecutionStatus::Failure),
        }
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![
            Port::output::<Image>("image"),
            Port::output::<f64>("capture_time"),
            Port::output::<f64>("capture_duration"),
        ])
    }
    fn setup_outputs(
        &mut self,
        interface: &mut dyn BlackboardOutputInterface,
    ) -> Result<(), NodeError> {
        self.output = interface.output::<Image>("image", Default::default())?;
        self.output_time = interface.output::<f64>("capture_time", Default::default())?;
        self.output_duration = interface.output::<f64>("capture_duration", Default::default())?;
        Ok(())
    }

    fn static_type() -> NodeType {
        "capture_node".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        let r = self.config.load_node_config(config);
        if let Some(capture) = self.capture.as_mut() {
            capture.set_config(self.config.capture.clone());
        }
        r
    }
}

#[cfg(feature = "betula_editor")]
mod ui_support {
    use super::*;
    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for CaptureNode {
        fn ui_title(&self) -> String {
            "capture 📷 ".to_owned()
        }

        fn ui_config(
            &mut self,
            ctx: &dyn UiNodeContext,
            ui: &mut egui::Ui,
            scale: f32,
        ) -> UiConfigResponse {
            let _ = (ctx, scale);

            let mut modified = false;
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Rate (hz)");
                    let r = ui.add(
                        egui::DragValue::new(&mut self.config.capture.rate)
                            .update_while_editing(false),
                    );
                    modified |= r.changed();
                });

                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new("➕")).clicked() {
                        self.config
                            .capture
                            .capture
                            .push(CaptureSpecification::default());
                        modified |= true;
                    }
                    if ui.add(egui::Button::new("➖")).clicked() {
                        if !self.config.capture.capture.is_empty() {
                            self.config
                                .capture
                                .capture
                                .truncate(self.config.capture.capture.len() - 1);
                            modified |= true;
                        }
                    }
                });

                ui.vertical(|ui| {
                    for (i, t) in self.config.capture.capture.iter_mut().enumerate() {
                        ui.heading(format!("Specification {i}"));
                        ui.horizontal(|ui| {
                            let mut match_width_enabled = t.match_width.is_some();
                            if ui
                                .checkbox(&mut match_width_enabled, "match_width")
                                .changed()
                            {
                                if match_width_enabled {
                                    t.match_width = Some(0);
                                } else {
                                    t.match_width = None;
                                }
                            }
                            if let Some(v) = t.match_width.as_mut() {
                                let r = ui.add(egui::DragValue::new(v).update_while_editing(false));
                                modified |= r.changed();
                            }

                            let mut match_height_enabled = t.match_height.is_some();
                            if ui
                                .checkbox(&mut match_height_enabled, "match_height")
                                .changed()
                            {
                                if match_height_enabled {
                                    t.match_height = Some(0);
                                } else {
                                    t.match_height = None;
                                }
                            }
                            if let Some(v) = t.match_height.as_mut() {
                                let r = ui.add(egui::DragValue::new(v).update_while_editing(false));
                                modified |= r.changed();
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("x offset");
                            let r =
                                ui.add(egui::DragValue::new(&mut t.x).update_while_editing(false));
                            modified |= r.changed();
                            ui.label("y offset");
                            let r =
                                ui.add(egui::DragValue::new(&mut t.y).update_while_editing(false));
                            modified |= r.changed();
                        });

                        ui.horizontal(|ui| {
                            ui.label("width");
                            let r = ui.add(
                                egui::DragValue::new(&mut t.width).update_while_editing(false),
                            );
                            modified |= r.changed();
                            ui.label("height");
                            let r = ui.add(
                                egui::DragValue::new(&mut t.height).update_while_editing(false),
                            );
                            modified |= r.changed();
                        });
                        ui.horizontal(|ui| {
                            ui.label("display");
                            let r = ui.add(
                                egui::DragValue::new(&mut t.display).update_while_editing(false),
                            );
                            modified |= r.changed();
                        });
                    }
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
                UiNodeCategory::Name("capture".to_owned()),
            ]
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..0
        }
    }
}
