use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

use crate::HotkeyBlackboard;
use crate::{Code, Hotkey, HotkeyToken, Modifiers};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
enum ModeType {
    Held,
    Toggle,
    OneShot,
}
impl ModeType {
    fn as_str(&self) -> &'static str {
        match self {
            ModeType::Held => "Held",
            ModeType::Toggle => "Toggle",
            ModeType::OneShot => "OneShot",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct HotkeyNodeConfig {
    alt_down: bool,
    shift_down: bool,
    control_down: bool,
    meta_down: bool,
    mode: ModeType,
    key: Code,
}
impl Default for HotkeyNodeConfig {
    fn default() -> Self {
        Self {
            alt_down: false,
            shift_down: false,
            control_down: false,
            meta_down: false,
            mode: ModeType::Toggle,
            key: Code::F10,
        }
    }
}
impl IsNodeConfig for HotkeyNodeConfig {}
impl HotkeyNodeConfig {
    pub fn to_hotkey(&self) -> Hotkey {
        let mut mods = Modifiers::default();
        if self.alt_down {
            mods.insert(Modifiers::ALT);
        }
        if self.shift_down {
            mods.insert(Modifiers::SHIFT);
        }
        if self.meta_down {
            mods.insert(Modifiers::META);
        }
        if self.control_down {
            mods.insert(Modifiers::CONTROL);
        }
        Hotkey::new(Some(mods), self.key)
    }
}

#[derive(Debug, Default)]
pub struct HotkeyNode {
    input: Input<HotkeyBlackboard>,
    config: HotkeyNodeConfig,
    token: Option<HotkeyToken>,
    last_count: usize,

    text_edit: Option<String>,
}

impl HotkeyNode {
    pub fn new() -> Self {
        HotkeyNode::default()
    }
}

impl Node for HotkeyNode {
    fn execute(&mut self, _ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        if self.token.is_none() {
            let hotkey = self.config.to_hotkey();
            self.token = Some(self.input.get()?.register(hotkey)?);
        }
        let token = self.token.as_ref().ok_or(format!("no token"))?;
        let new_count = token.state.depress_usize();
        let active = match self.config.mode {
            ModeType::Held => token.is_pressed(),
            ModeType::Toggle => token.depress_count() % 2 == 1,
            ModeType::OneShot => new_count != self.last_count,
        };
        self.last_count = new_count;

        if active {
            Ok(ExecutionStatus::Success)
        } else {
            Ok(ExecutionStatus::Failure)
        }
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![Port::input::<HotkeyBlackboard>("hotkey")])
    }
    fn setup_inputs(
        &mut self,
        interface: &mut dyn BlackboardInputInterface,
    ) -> Result<(), NodeError> {
        self.input = interface.input::<HotkeyBlackboard>("hotkey")?;
        Ok(())
    }

    fn static_type() -> NodeType {
        "hotkey_node".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        let old_config = self.config;
        self.config.load_node_config(config)?;
        if old_config != self.config {
            self.token = None;
        }
        Ok(())
    }
}

#[cfg(feature = "betula_editor")]
mod ui_support {
    use super::*;
    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for HotkeyNode {
        fn ui_title(&self) -> String {
            "hotkey 🔥🔑".to_owned()
        }

        fn ui_config(
            &mut self,
            ctx: &dyn UiNodeContext,
            ui: &mut egui::Ui,
            scale: f32,
        ) -> UiConfigResponse {
            let _ = ctx;
            let mut modified = false;

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    modified |= ui.checkbox(&mut self.config.alt_down, "alt").changed();
                    modified |= ui
                        .checkbox(&mut self.config.control_down, "control")
                        .changed();
                    modified |= ui.checkbox(&mut self.config.shift_down, "shift").changed();
                });
                ui.horizontal(|ui| {
                    modified |= ui.checkbox(&mut self.config.meta_down, "meta").changed();
                    let d = &mut self.config.mode;
                    let z = egui::ComboBox::from_id_source("hotkey_mode")
                        .width(0.0)
                        .selected_text(d.as_str())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(d, ModeType::Held, ModeType::Held.as_str())
                                .on_hover_text("Success while the key is held down.")
                                | ui.selectable_value(
                                    d,
                                    ModeType::Toggle,
                                    ModeType::Toggle.as_str(),
                                )
                                .on_hover_text("Success and failure are toggled by the key.")
                                | ui.selectable_value(
                                    d,
                                    ModeType::OneShot,
                                    ModeType::OneShot.as_str(),
                                )
                                .on_hover_text(
                                    "Success is returned for one execution cycle after key press.",
                                )
                        });
                    modified |= z.inner.unwrap_or(z.response).changed()
                });

                // modified |= ui.checkbox(&mut self.config.toggle, "toggle").changed();

                let current_str = format!("{:}", self.config.key);
                if let Some(ref mut edit_string) = self.text_edit {
                    use std::str::FromStr;
                    let is_valid = Code::from_str(&edit_string).is_ok();
                    let text_color = if is_valid {
                        None
                    } else {
                        Some(egui::Color32::RED)
                    };
                    let hint_text = "KeyC, F10, PageUp: uievents-code";
                    let response = ui.add(
                        egui::TextEdit::singleline(edit_string)
                            .hint_text(hint_text)
                            .min_size(egui::vec2(100.0 * scale, 0.0))
                            .text_color_opt(text_color),
                    );
                    if response.on_hover_text(hint_text).lost_focus() {
                        if let Ok(v) = Code::from_str(&edit_string) {
                            modified = true;
                            self.config.key = v;
                            self.text_edit = None;
                        }
                    }
                } else {
                    let r = ui.label(&current_str);
                    if r.on_hover_text("click to modify").clicked() {
                        self.text_edit = Some(current_str);
                    }
                }
            });

            if modified {
                UiConfigResponse::Changed
            } else {
                UiConfigResponse::UnChanged
            }
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("conditional".to_owned()),
                UiNodeCategory::Name("hotkey".to_owned()),
            ]
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..0
        }
    }
}
