use crate::{ui::UiNodeCategory, UiConfigResponse, UiNode, UiNodeContext};
use egui::Ui;

use betula_common::nodes;

impl UiNode for nodes::DelayNode {
    fn ui_title(&self) -> String {
        "delay ⏱".to_owned()
    }

    fn ui_config(&mut self, ctx: &dyn UiNodeContext, ui: &mut Ui, _scale: f32) -> UiConfigResponse {
        let _ = ctx;
        let mut ui_response = UiConfigResponse::UnChanged;
        ui.horizontal(|ui| {
            ui.label("Delay: ");

            let speed = if self.config.interval < 1.0 {
                0.001
            } else if self.config.interval < 10.0 {
                0.01
            } else {
                0.1
            };

            let r = ui.add(
                egui::DragValue::new(&mut self.config.interval)
                    .clamp_range(0.0f64..=(24.0 * 60.0 * 60.0))
                    .speed(speed)
                    .custom_formatter(|v, _| {
                        if v < 10.0 {
                            format!("{:.0} ms", v * 1000.0)
                        } else if v < 60.0 {
                            format!("{:.3} s", v)
                        } else {
                            format!("{:.0} s", v)
                        }
                    })
                    .custom_parser(|s| {
                        let parts: Vec<&str> = s.split(' ').collect();
                        let value = parts[0].parse::<f64>().ok()?;
                        if let Some(scale) = parts.get(1) {
                            if *scale == "ms" {
                                Some(value * 0.001)
                            } else if *scale == "s" {
                                Some(value)
                            } else if *scale == "m" {
                                Some(value * 60.0)
                            } else {
                                None
                            }
                        } else {
                            Some(value)
                        }
                    })
                    .update_while_editing(false),
            );

            if r.changed() {
                // println!("Changed! now: {}", self.config.interval);
                ui_response = UiConfigResponse::Changed;
            }
        });

        ui_response
    }
    fn ui_child_range(&self) -> std::ops::Range<usize> {
        0..1
    }

    fn ui_category() -> Vec<UiNodeCategory> {
        vec![
            UiNodeCategory::Folder("common".to_owned()),
            UiNodeCategory::Name("delay".to_owned()),
        ]
    }
}

impl UiNode for nodes::TimeNode {
    fn ui_title(&self) -> String {
        "time 🕓".to_owned()
    }

    fn ui_child_range(&self) -> std::ops::Range<usize> {
        0..0
    }
    fn ui_category() -> Vec<UiNodeCategory> {
        vec![
            UiNodeCategory::Folder("common".to_owned()),
            UiNodeCategory::Name("time".to_owned()),
        ]
    }
}

impl UiNode for nodes::ParallelNode {
    fn ui_title(&self) -> String {
        "parallel 🔀".to_owned()
    }

    fn ui_config(&mut self, ctx: &dyn UiNodeContext, ui: &mut Ui, _scale: f32) -> UiConfigResponse {
        let children_count = ctx.children_count();
        let mut ui_response = UiConfigResponse::UnChanged;

        if self.config.success_threshold > children_count {
            self.config.success_threshold = children_count;
            ui_response = UiConfigResponse::Changed;
        }
        ui.horizontal(|ui| {
            ui.label("Threshold: ");
            let r = ui.add(
                egui::DragValue::new(&mut self.config.success_threshold)
                    .clamp_range(0..=children_count)
                    .update_while_editing(false),
            );

            if r.changed() {
                ui_response = UiConfigResponse::Changed;
            }
        });

        ui_response
    }

    fn ui_category() -> Vec<UiNodeCategory> {
        vec![
            UiNodeCategory::Folder("logic".to_owned()),
            UiNodeCategory::Name("parallel".to_owned()),
        ]
    }
}
