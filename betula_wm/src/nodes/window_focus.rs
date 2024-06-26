use betula_core::node_prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WindowFocusNodeConfig {
    pub matches: Vec<String>,
}
impl IsNodeConfig for WindowFocusNodeConfig {}

use crate::WindowFocusRetriever;

#[derive(Debug, Default)]
pub struct WindowFocusNode {
    pub config: WindowFocusNodeConfig,
    focus: WindowFocusRetriever,
    matches: Vec<Regex>,

    #[cfg(feature = "betula_editor")]
    regex_editor: Option<(usize, String)>,
}

impl WindowFocusNode {
    pub fn new() -> Self {
        WindowFocusNode::default()
    }
}

impl Node for WindowFocusNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        let name = self.focus.process_name()?;
        for re in &self.matches {
            if re.is_match(&name) {
                if ctx.children() == 0 {
                    return Ok(ExecutionStatus::Success);
                } else if ctx.children() == 1 {
                    return ctx.run(0);
                } else if ctx.children() > 1 {
                    return Err(format!("{:?} had more than one child", Self::static_type()).into());
                }
            }
        }
        Ok(ExecutionStatus::Failure)
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        self.config.load_node_config(config)?;
        self.matches.clear();
        for v in self.config.matches.iter() {
            self.matches.push(Regex::new(v)?);
        }

        Ok(())
    }
    fn static_type() -> NodeType {
        "program_focus".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}

#[cfg(feature = "betula_editor")]
pub mod ui_support {
    use super::*;
    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for WindowFocusNode {
        fn ui_title(&self) -> String {
            "window_focus 🗖".to_owned()
        }

        fn ui_config(
            &mut self,
            ctx: &dyn UiNodeContext,
            ui: &mut egui::Ui,
            scale: f32,
        ) -> UiConfigResponse {
            let _ = ctx;
            let mut ui_response = UiConfigResponse::UnChanged;

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new("➕")).clicked() {
                        self.config.matches.push("".to_owned());
                        ui_response = UiConfigResponse::Changed;
                    }
                    if ui.add(egui::Button::new("➖")).clicked() {
                        if !self.config.matches.is_empty() {
                            let new_length = self.config.matches.len() - 1;
                            self.config.matches.truncate(new_length);

                            if let Some(edit_i) = self.regex_editor.as_ref().map(|z| z.0) {
                                if edit_i >= new_length {
                                    self.regex_editor = None;
                                }
                            }
                            ui_response = UiConfigResponse::Changed;
                        }
                    }
                });
                ui.vertical(|ui| {
                    let edit_i = self.regex_editor.as_ref().map(|z| z.0);
                    for (i, t) in self.config.matches.iter_mut().enumerate() {
                        if edit_i == Some(i) {
                            if let Some((_i, ref mut edit_t)) = self.regex_editor.as_mut() {
                                let is_valid = Regex::new(&edit_t).is_ok();
                                let text_color = if is_valid {
                                    None
                                } else {
                                    Some(egui::Color32::RED)
                                };
                                let hint_text = "regex";
                                let response = ui.add(
                                    egui::TextEdit::singleline(edit_t)
                                        .hint_text(hint_text)
                                        .min_size(egui::vec2(100.0 * scale, 0.0))
                                        .text_color_opt(text_color),
                                );
                                if response.lost_focus() {
                                    if is_valid {
                                        ui_response = UiConfigResponse::Changed;
                                        *t = edit_t.clone();
                                        self.regex_editor = None;
                                    }
                                }
                            }
                        } else {
                            let r = ui.add(
                                egui::Label::new(format!(
                                    "{}",
                                    if t.is_empty() { "/regex/" } else { &t }
                                ))
                                .wrap(false),
                            );
                            if r.clicked() {
                                self.regex_editor = Some((i, t.to_owned()));
                            }
                        }
                    }
                });
            });

            ui_response
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..1
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("conditional".to_owned()),
                UiNodeCategory::Name("window_focus".to_owned()),
            ]
        }
    }
}
