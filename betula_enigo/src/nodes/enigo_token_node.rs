use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

use crate::{EnigoBlackboard, EnigoRunner};

use enigo::agent::Token;
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EnigoTokenNodeConfig {
    execute_async: bool,
    tokens: Vec<Token>,
}
impl IsNodeConfig for EnigoTokenNodeConfig {}

#[derive(Debug, Default)]
pub struct EnigoTokenNode {
    input: Input<EnigoBlackboard>,
    pub config: EnigoTokenNodeConfig,
}

impl EnigoTokenNode {
    pub fn new() -> Self {
        EnigoTokenNode::default()
    }
}

impl Node for EnigoTokenNode {
    fn execute(&mut self, _ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        let mut interface = self.input.get()?;
        if self.config.execute_async {
            interface.execute_async(&self.config.tokens)?;
        } else {
            interface.execute(&self.config.tokens)?;
        }
        Ok(ExecutionStatus::Success)
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![Port::input::<EnigoBlackboard>("enigo")])
    }
    fn setup_inputs(
        &mut self,
        interface: &mut dyn BlackboardInputInterface,
    ) -> Result<(), NodeError> {
        self.input = interface.input::<EnigoBlackboard>("enigo")?;
        Ok(())
    }

    fn static_type() -> NodeType {
        "enigo_token".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        self.config.load_node_config(config)
    }
}

#[cfg(feature = "betula_egui")]
mod ui_support {
    use super::*;
    use betula_egui::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for EnigoTokenNode {
        fn ui_title(&self) -> String {
            "enigo_token 🖱🖮 ".to_owned()
        }

        fn ui_config(
            &mut self,
            ctx: &dyn UiNodeContext,
            ui: &mut egui::Ui,
            _scale: f32,
        ) -> UiConfigResponse {
            let _ = ctx;
            let mut ui_response = UiConfigResponse::UnChanged;
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new("➕")).clicked() {
                        self.config
                            .tokens
                            .push(enigo::agent::Token::Text("".to_owned()));
                        ui_response = UiConfigResponse::Changed;
                    }
                    if ui.add(egui::Button::new("➖")).clicked() {
                        if !self.config.tokens.is_empty() {
                            self.config.tokens.truncate(self.config.tokens.len() - 1);
                            ui_response = UiConfigResponse::Changed;
                        }
                    }
                    let r = ui.checkbox(&mut self.config.execute_async, "Async");
                    if r.changed() {
                        ui_response = UiConfigResponse::Changed;
                    }
                });

                ui.vertical(|ui| {
                    for (i, t) in self.config.tokens.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            let options = [
                                ("Text", Token::Text("".to_owned())),
                                (
                                    "Key",
                                    Token::Key(enigo::Key::Unicode('a'), enigo::Direction::Click),
                                ),
                            ];
                            // let alternatives = ["Text", "Key"];
                            let mut selected = match t {
                                Token::Text(_) => 0,
                                Token::Key(_, _) => 1,
                                _ => unreachable!(),
                            };
                            let z = egui::ComboBox::from_id_source(i)
                                .width(0.0)
                                .selected_text(format!("{:?}", selected))
                                .show_index(ui, &mut selected, options.len(), |i| options[i].0);
                            if z.changed() {
                                *t = options[selected].1.clone();
                                ui_response = UiConfigResponse::Changed;
                            }
                            match t {
                                Token::Text(ref mut v) => {
                                    let response = ui.add(egui::TextEdit::singleline(v));
                                    if response.changed() {
                                        ui_response = UiConfigResponse::Changed;
                                    }
                                }
                                Token::Key(ref mut k, ref mut d) => {
                                    let z = egui::ComboBox::from_id_source(format!("keydir{i}"))
                                        .width(0.0)
                                        .selected_text(format!("{:?}", d))
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(
                                                d,
                                                enigo::Direction::Press,
                                                "⬇ Press",
                                            ) | ui.selectable_value(
                                                d,
                                                enigo::Direction::Release,
                                                "⬆ Release",
                                            ) | ui.selectable_value(
                                                d,
                                                enigo::Direction::Click,
                                                "❇ Click",
                                            )
                                        });
                                    let response = z.inner.unwrap_or(z.response);
                                    if response.changed() {
                                        ui_response = UiConfigResponse::Changed;
                                    }
                                    // There's 95 options here :(

                                    let y = egui::ComboBox::from_id_source(format!("key{i}"))
                                        .selected_text(format!("{:?}", k))
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(k, enigo::Key::Alt, "Alt")
                                                | ui.selectable_value(k, enigo::Key::Space, "Space")
                                                | ui.selectable_value(
                                                    k,
                                                    if matches!(k, enigo::Key::Unicode(_)) {
                                                        *k
                                                    } else {
                                                        enigo::Key::Unicode('a')
                                                    },
                                                    "Unicode",
                                                )
                                        });
                                    let response = y.inner.unwrap_or(y.response);
                                    if response.changed() {
                                        ui_response = UiConfigResponse::Changed;
                                    }
                                    match k {
                                        enigo::Key::Unicode(ref mut c) => {
                                            let mut buffer = format!("{c}");
                                            let output = egui::TextEdit::singleline(&mut buffer)
                                                .hint_text("select text to edit")
                                                .char_limit(1)
                                                .show(ui);
                                            // output.response
                                            if output
                                                .response
                                                .on_hover_text("select the character, replace it")
                                                .changed()
                                            {
                                                ui_response = UiConfigResponse::Changed;
                                                if let Some(v) = buffer.chars().next() {
                                                    *c = v;
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                _ => {}
                            }
                        });
                    }
                });
            });

            ui_response
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("action".to_owned()),
                UiNodeCategory::Name("enigo".to_owned()),
            ]
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..0
        }
    }
}
