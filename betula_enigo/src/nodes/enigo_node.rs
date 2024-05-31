use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

use crate::{load_preset_directory, EnigoPreset, EnigoTokens};

use enigo::agent::Token;
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EnigoNodeConfig {
    tokens: Vec<Token>,
    preset: Option<Vec<String>>,
}
impl IsNodeConfig for EnigoNodeConfig {}

#[derive(Debug, Default)]
pub struct EnigoNode {
    input: Input<EnigoTokens>,

    output: Output<EnigoTokens>,

    pub config: EnigoNodeConfig,

    /// The directory from which the patterns are loaded.
    directory: Option<std::path::PathBuf>,

    /// The available patterns for selection.
    presets: Vec<EnigoPreset>,

    /// True if the preset needs to be reloaded.
    preset_dirty: bool,
}

impl EnigoNode {
    pub fn new() -> Self {
        EnigoNode::default()
    }

    pub fn load_presets(&mut self) -> Result<(), NodeError> {
        if let Some(dir) = &self.directory {
            let mut dir = dir.clone();
            dir.push("enigo_node");
            self.presets = load_preset_directory(&dir)?;
        }
        Ok(())
    }

    pub fn apply_preset(&mut self) -> Result<(), NodeError> {
        if let Some(desired) = self.config.preset.as_ref() {
            if let Some(entry) = self.presets.iter().find(|z| &z.index == desired) {
                self.config.tokens.clone_from(&entry.info.actions);
            } else {
                return Err(format!("Could not find desired preset: {desired:?}").into());
            }
        }
        Ok(())
    }
}

impl Node for EnigoNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        if self.preset_dirty {
            self.load_presets()?;
            self.apply_preset()?;
            self.preset_dirty = false;
        }
        let tokens = if let Ok(mut tokens) = self.input.get() {
            tokens
                .0
                .drain(..)
                .chain(self.config.tokens.iter().cloned())
                .collect()
        } else {
            self.config.tokens.clone()
        };
        self.output.set(EnigoTokens(tokens))?;

        ctx.decorate_or(ExecutionStatus::Success)
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![
            Port::output::<EnigoTokens>("tokens"),
            Port::input::<EnigoTokens>("tokens"),
        ])
    }

    fn setup_inputs(
        &mut self,
        interface: &mut dyn BlackboardInputInterface,
    ) -> Result<(), NodeError> {
        self.input = interface.input::<EnigoTokens>("tokens")?;
        Ok(())
    }

    fn setup_outputs(
        &mut self,
        interface: &mut dyn BlackboardOutputInterface,
    ) -> Result<(), NodeError> {
        self.output = interface.output::<EnigoTokens>("tokens", Default::default())?;
        Ok(())
    }

    fn static_type() -> NodeType {
        "enigo_node".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        let preset_before = self.config.preset.clone();
        let r = self.config.load_node_config(config);
        self.preset_dirty = preset_before != self.config.preset && self.config.preset.is_some();
        let _ = self.apply_preset();
        r
    }

    fn set_directory(&mut self, directory: Option<&std::path::Path>) {
        self.directory = directory.map(|v| v.to_owned());
        let _ = self.load_presets();
    }
}

#[cfg(feature = "betula_editor")]
mod ui_support {
    use super::*;
    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};
    use betula_editor::{menu_node_recurser, UiMenuNode, UiMenuTree};
    use enigo::{Axis, Coordinate, Direction};

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

    fn axis_to_str(d: Axis) -> &'static str {
        match d {
            Axis::Horizontal => "Horizontal",
            Axis::Vertical => "Vertical",
        }
    }
    fn axis_ui(
        id_source: impl std::hash::Hash,
        d: &mut enigo::Axis,
        ui: &mut egui::Ui,
    ) -> egui::Response {
        let z = egui::ComboBox::from_id_source(id_source)
            .width(0.0)
            .selected_text(format!("{:}", axis_to_str(*d)))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    d,
                    enigo::Axis::Horizontal,
                    axis_to_str(enigo::Axis::Horizontal),
                ) | ui.selectable_value(
                    d,
                    enigo::Axis::Vertical,
                    axis_to_str(enigo::Axis::Vertical),
                )
            });
        z.inner.unwrap_or(z.response)
    }

    impl UiNode for EnigoNode {
        fn ui_title(&self) -> String {
            if let Some(preset) = &self.config.preset {
                preset.join(".")
            } else {
                "enigo".to_owned()
            }
        }
        fn ui_icon(&self, ui: &mut egui::Ui, desired_size: egui::Vec2) {
            let _ = desired_size;
            ui.add(egui::Label::new("🖱🖮").selectable(false));
        }

        fn ui_config(
            &mut self,
            ctx: &dyn UiNodeContext,
            ui: &mut egui::Ui,
            scale: f32,
        ) -> UiConfigResponse {
            let _ = ctx;

            let mut preset_modified = false;
            let mut non_preset_modified = false;
            let mut token_modified = false;

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let label = if let Some(index) = self.config.preset.clone() {
                        index.join(".")
                    } else {
                        "Select...".to_owned()
                    };

                    ui.menu_button(label, |ui| {
                        // Convert the pattern library to the menu tree.
                        type MenuType<'a> = UiMenuNode<String, &'a EnigoPreset>;
                        type TreeType<'a> = UiMenuTree<String, &'a EnigoPreset>;
                        let mut root = TreeType::new();
                        for pattern in self.presets.iter() {
                            let index_into = &pattern.index[0..pattern.index.len() - 1];
                            let element = {
                                let mut element = &mut root;
                                for sub in index_into {
                                    element = element
                                        .entry(sub.clone())
                                        .or_insert_with(|| MenuType::SubElements(TreeType::new()))
                                        .sub_elements();
                                }
                                element
                            };
                            element.insert(
                                pattern.index.last().unwrap().clone(),
                                MenuType::Value(pattern),
                            );
                        }

                        if let Some(entry) = menu_node_recurser(&root, ui) {
                            self.config.preset = Some(entry.index.clone());
                            self.config.tokens = entry.info.actions.clone();
                            preset_modified |= true;
                            ui.close_menu();
                        }
                    });

                    if ui
                        .button("🔃")
                        .on_hover_text("Reload presets from directory.")
                        .clicked()
                    {
                        let patterns = self.load_presets();
                        if let Err(e) = patterns {
                            println!("Error loading presets: {:?}", e)
                        }
                        ui.close_menu();
                    }

                    if ui.add(egui::Button::new("➕")).clicked() {
                        self.config
                            .tokens
                            .push(enigo::agent::Token::Text("".to_owned()));
                        non_preset_modified = true;
                    }
                    if ui.add(egui::Button::new("➖")).clicked() {
                        if !self.config.tokens.is_empty() {
                            self.config.tokens.truncate(self.config.tokens.len() - 1);
                            non_preset_modified = true;
                        }
                    }
                });

                let mut change_token_position = None;
                ui.vertical(|ui| {
                    let total = self.config.tokens.len();
                    for (i, t) in self.config.tokens.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(i != total - 1, egui::Button::new("⮋"))
                                .clicked()
                            {
                                change_token_position = Some((i, 1));
                            }
                            if ui.add_enabled(i != 0, egui::Button::new("⮉")).clicked() {
                                change_token_position = Some((i, -1));
                            }
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
                                ("📜Scroll", Token::Scroll(0, enigo::Axis::Vertical)),
                            ];
                            // let alternatives = ["Text", "Key"];
                            let mut selected = match t {
                                Token::Text(_) => 0,
                                Token::Key(_, _) => 1,
                                Token::Button(_, _) => 2,
                                Token::MoveMouse(_, _, _) => 3,
                                Token::Scroll(_, _) => 4,
                                _ => unreachable!(),
                            };
                            let z = egui::ComboBox::from_id_source(i)
                                .width(0.0)
                                .selected_text(format!("{:?}", selected))
                                .show_index(ui, &mut selected, options.len(), |i| options[i].0);
                            if z.changed() {
                                *t = options[selected].1.clone();
                                token_modified = true;
                            }
                            match t {
                                Token::Text(ref mut v) => {
                                    let text = "text to be inserted, doesn't work for shortcuts";
                                    let response = ui.add(
                                        egui::TextEdit::singleline(v)
                                            .hint_text(text)
                                            .min_size(egui::vec2(100.0 * scale, 0.0)),
                                    );
                                    token_modified |= response.on_hover_text(text).changed();
                                }
                                Token::Key(ref mut k, ref mut d) => {
                                    let response = direction_ui(format!("keydir{i}"), d, ui);
                                    token_modified |= response.changed();
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
                                    token_modified |= response.changed();
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
                                                    token_modified = true;
                                                    *c = v;
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                Token::Button(ref mut b, ref mut d) => {
                                    let response = direction_ui(format!("buttondir{i}"), d, ui);
                                    token_modified |= response.changed();
                                    let response = button_ui(format!("button{i}"), b, ui);
                                    token_modified |= response.changed();
                                }
                                Token::MoveMouse(ref mut x, ref mut y, ref mut c) => {
                                    ui.label("x");
                                    let r =
                                        ui.add(egui::DragValue::new(x).update_while_editing(false));
                                    token_modified |= r.changed();
                                    ui.label("y");
                                    let r =
                                        ui.add(egui::DragValue::new(y).update_while_editing(false));
                                    token_modified |= r.changed();
                                    let r = coordinate_ui(format!("coordinate{i}"), c, ui);
                                    token_modified |= r.changed();
                                }
                                Token::Scroll(ref mut v, ref mut c) => {
                                    let r =
                                        ui.add(egui::DragValue::new(v).update_while_editing(false));
                                    token_modified |= r.changed();
                                    let response = axis_ui(format!("axis{i}"), c, ui);
                                    token_modified |= response.changed();
                                }
                                _ => {}
                            }
                        });
                    }
                });
                if let Some((pos, dir)) = change_token_position {
                    let new_pos = (pos as isize + dir) as usize;
                    self.config.tokens.swap(pos, new_pos);
                    token_modified |= true;
                }
            });

            if token_modified {
                // This is no longer a preset, it has been modified.
                self.config.preset = None;
            }
            if preset_modified {
                self.preset_dirty = true;
            }

            if preset_modified || non_preset_modified || token_modified {
                UiConfigResponse::Changed
            } else {
                UiConfigResponse::UnChanged
            }
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("action".to_owned()),
                UiNodeCategory::Name("enigo".to_owned()),
            ]
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..1
        }
    }
}
