use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

use crate::EnigoBlackboard;

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
    use enigo::{Coordinate, Direction};

    fn direction_to_str(d: Direction) -> &'static str {
        match d {
            Direction::Press => "⬇",
            Direction::Release => "⬆",
            Direction::Click => "❇",
        }
    }

    fn direction_ui(
        id_source: impl std::hash::Hash,
        d: &mut Direction,
        ui: &mut egui::Ui,
    ) -> egui::Response {
        let z = egui::ComboBox::from_id_source(id_source)
            .width(0.0)
            .selected_text(format!("{:}", direction_to_str(*d)))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    d,
                    enigo::Direction::Press,
                    direction_to_str(enigo::Direction::Press),
                )
                .on_hover_text("press")
                    | ui.selectable_value(
                        d,
                        enigo::Direction::Release,
                        direction_to_str(enigo::Direction::Release),
                    )
                    .on_hover_text("release")
                    | ui.selectable_value(
                        d,
                        enigo::Direction::Click,
                        direction_to_str(enigo::Direction::Click),
                    )
                    .on_hover_text("click")
            });
        z.inner.unwrap_or(z.response)
    }

    fn coordinate_to_str(d: Coordinate) -> &'static str {
        match d {
            Coordinate::Abs => "Abs",
            Coordinate::Rel => "Rel",
        }
    }
    fn coordinate_ui(
        id_source: impl std::hash::Hash,
        d: &mut Coordinate,
        ui: &mut egui::Ui,
    ) -> egui::Response {
        let z = egui::ComboBox::from_id_source(id_source)
            .width(0.0)
            .selected_text(format!("{:}", coordinate_to_str(*d)))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    d,
                    enigo::Coordinate::Abs,
                    coordinate_to_str(enigo::Coordinate::Abs),
                )
                .on_hover_text("absolute coordinates")
                    | ui.selectable_value(
                        d,
                        enigo::Coordinate::Rel,
                        coordinate_to_str(enigo::Coordinate::Rel),
                    )
                    .on_hover_text("relative coordinates")
            });
        z.inner.unwrap_or(z.response)
    }

    fn button_to_str(d: enigo::Button) -> &'static str {
        match d {
            enigo::Button::Left => "Left",
            enigo::Button::Middle => "Middle",
            enigo::Button::Right => "Right",
            enigo::Button::Back => "Back",
            enigo::Button::Forward => "Forward",
            enigo::Button::ScrollUp => "ScrollUp",
            enigo::Button::ScrollDown => "ScrollDown",
            enigo::Button::ScrollLeft => "ScrollLeft",
            enigo::Button::ScrollRight => "ScrollRight",
        }
    }
    fn button_ui(
        id_source: impl std::hash::Hash,
        d: &mut enigo::Button,
        ui: &mut egui::Ui,
    ) -> egui::Response {
        let z = egui::ComboBox::from_id_source(id_source)
            .width(0.0)
            .selected_text(format!("{:}", button_to_str(*d)))
            .show_ui(ui, |ui| {
                ui.selectable_value(d, enigo::Button::Left, button_to_str(enigo::Button::Left))
                    | ui.selectable_value(
                        d,
                        enigo::Button::Middle,
                        button_to_str(enigo::Button::Middle),
                    )
                    | ui.selectable_value(
                        d,
                        enigo::Button::Right,
                        button_to_str(enigo::Button::Right),
                    )
                    | ui.selectable_value(
                        d,
                        enigo::Button::Back,
                        button_to_str(enigo::Button::Back),
                    )
                    | ui.selectable_value(
                        d,
                        enigo::Button::Forward,
                        button_to_str(enigo::Button::Forward),
                    )
                    | ui.selectable_value(
                        d,
                        enigo::Button::ScrollUp,
                        button_to_str(enigo::Button::ScrollUp),
                    )
                    | ui.selectable_value(
                        d,
                        enigo::Button::ScrollDown,
                        button_to_str(enigo::Button::ScrollDown),
                    )
                    | ui.selectable_value(
                        d,
                        enigo::Button::ScrollLeft,
                        button_to_str(enigo::Button::ScrollLeft),
                    )
                    | ui.selectable_value(
                        d,
                        enigo::Button::ScrollRight,
                        button_to_str(enigo::Button::ScrollRight),
                    )
            });
        z.inner.unwrap_or(z.response)
    }

    impl UiNode for EnigoTokenNode {
        fn ui_title(&self) -> String {
            "enigo_token 🖱🖮 ".to_owned()
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
                                ("📖Text", Token::Text("".to_owned())),
                                (
                                    "🖮 Key",
                                    Token::Key(enigo::Key::Unicode('a'), enigo::Direction::Click),
                                ),
                                (
                                    "🖱 Button",
                                    Token::Button(enigo::Button::Left, enigo::Direction::Click),
                                ),
                                ("👆Mouse", Token::MoveMouse(0, 0, enigo::Coordinate::Rel)),
                            ];
                            // let alternatives = ["Text", "Key"];
                            let mut selected = match t {
                                Token::Text(_) => 0,
                                Token::Key(_, _) => 1,
                                Token::Button(_, _) => 2,
                                Token::MoveMouse(_, _, _) => 3,
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
                                    let text = "text to be inserted, doesn't work for shortcuts";
                                    let response = ui.add(
                                        egui::TextEdit::singleline(v)
                                            .hint_text(text)
                                            .min_size(egui::vec2(100.0 * scale, 0.0)),
                                    );
                                    if response.on_hover_text(text).changed() {
                                        ui_response = UiConfigResponse::Changed;
                                    }
                                }
                                Token::Key(ref mut k, ref mut d) => {
                                    let response = direction_ui(format!("keydir{i}"), d, ui);
                                    if response.changed() {
                                        ui_response = UiConfigResponse::Changed;
                                    }
                                    // There's 96 options here :(
                                    // https://docs.rs/enigo/latest/enigo/enum.Key.html#variant.Unicode

                                    let y = egui::ComboBox::from_id_source(format!("key{i}"))
                                        .selected_text(format!("{:?}", k))
                                        .height(10000.0)
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(
                                                k,
                                                if matches!(k, enigo::Key::Unicode(_)) {
                                                    *k
                                                } else {
                                                    enigo::Key::Unicode('a')
                                                },
                                                "Unicode",
                                            ) | ui.label("Modifiers")
                                                | ui.selectable_value(k, enigo::Key::Alt, "Alt")
                                                | ui.selectable_value(k, enigo::Key::Meta, "Meta")
                                                | ui.selectable_value(
                                                    k,
                                                    enigo::Key::LShift,
                                                    "LeftShift",
                                                )
                                                | ui.selectable_value(
                                                    k,
                                                    enigo::Key::LControl,
                                                    "LeftControl",
                                                )
                                                | ui.selectable_value(
                                                    k,
                                                    enigo::Key::RShift,
                                                    "RightShift",
                                                )
                                                | ui.selectable_value(
                                                    k,
                                                    enigo::Key::RControl,
                                                    "RightControl",
                                                )
                                                | ui.label("Whitespace")
                                                | ui.selectable_value(k, enigo::Key::Space, "Space")
                                                | ui.selectable_value(
                                                    k,
                                                    enigo::Key::Backspace,
                                                    "Backspace",
                                                )
                                                | ui.selectable_value(
                                                    k,
                                                    enigo::Key::Escape,
                                                    "Escape",
                                                )
                                                | ui.selectable_value(k, enigo::Key::Tab, "Tab")
                                                | ui.selectable_value(
                                                    k,
                                                    enigo::Key::Return,
                                                    "Return",
                                                )
                                                | ui.label("Arrow")
                                                | ui.selectable_value(
                                                    k,
                                                    enigo::Key::LeftArrow,
                                                    "LeftArrow",
                                                )
                                                | ui.selectable_value(
                                                    k,
                                                    enigo::Key::RightArrow,
                                                    "RightArrow",
                                                )
                                                | ui.selectable_value(
                                                    k,
                                                    enigo::Key::DownArrow,
                                                    "DownArrow",
                                                )
                                                | ui.selectable_value(
                                                    k,
                                                    enigo::Key::UpArrow,
                                                    "UpArrow",
                                                )
                                                | ui.label("Misc")
                                                | ui.selectable_value(k, enigo::Key::Print, "Print")
                                                | ui.selectable_value(
                                                    k,
                                                    enigo::Key::PageUp,
                                                    "PageUp",
                                                )
                                                | ui.selectable_value(
                                                    k,
                                                    enigo::Key::PageDown,
                                                    "PageDown",
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
                                                .desired_width(15.0)
                                                .show(ui);
                                            if output
                                                .response
                                                .on_hover_text("select the character, replace it")
                                                .changed()
                                            {
                                                if let Some(v) = buffer.chars().next() {
                                                    ui_response = UiConfigResponse::Changed;
                                                    *c = v;
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                Token::Button(ref mut b, ref mut d) => {
                                    let response = direction_ui(format!("buttondir{i}"), d, ui);
                                    if response.changed() {
                                        ui_response = UiConfigResponse::Changed;
                                    }
                                    let response = button_ui(format!("button{i}"), b, ui);
                                    if response.changed() {
                                        ui_response = UiConfigResponse::Changed;
                                    }
                                }
                                Token::MoveMouse(ref mut x, ref mut y, ref mut c) => {
                                    ui.label("x");
                                    let r =
                                        ui.add(egui::DragValue::new(x).update_while_editing(false));
                                    if r.changed() {
                                        ui_response = UiConfigResponse::Changed;
                                    }
                                    ui.label("y");
                                    let r =
                                        ui.add(egui::DragValue::new(y).update_while_editing(false));
                                    if r.changed() {
                                        ui_response = UiConfigResponse::Changed;
                                    }
                                    let response = coordinate_ui(format!("coordinate{i}"), c, ui);
                                    if response.changed() {
                                        ui_response = UiConfigResponse::Changed;
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
