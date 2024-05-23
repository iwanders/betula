use betula_common::callback::{CallbacksBlackboard, Ticket};
use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

use crate::ImageCursor;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ImageWriteCursorNodeConfig {
    path: String,
}
impl IsNodeConfig for ImageWriteCursorNodeConfig {}

#[derive(Default)]
pub struct ImageWriteCursorNode {
    input_image_cursor_cb: Input<CallbacksBlackboard<ImageCursor>>,
    /// The directory from which the patterns are loaded.
    directory: Option<std::path::PathBuf>,

    pub config: ImageWriteCursorNodeConfig,

    ticket: Option<Ticket<ImageCursor>>,
}

impl std::fmt::Debug for ImageWriteCursorNode {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "ImageWriteCursorNode")
    }
}

impl Node for ImageWriteCursorNode {
    fn execute(&mut self, _ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        if self.ticket.is_none() {
            let callback_value = self.input_image_cursor_cb.get()?;
            let callback_interface = callback_value
                .callbacks()
                .ok_or(format!("callbacks not populated yet"))?;
            if self.config.path.is_empty() {
                self.ticket = Some(callback_interface.register(|img| {}));
            } else {
                let config_path = std::path::PathBuf::from(&self.config.path);
                let final_path = if config_path.is_absolute() {
                    config_path
                } else {
                    let mut dir = self
                        .directory
                        .as_ref()
                        .ok_or("directory path isn't set yet")?
                        .clone();
                    dir.push(config_path);
                    dir
                };

                self.ticket = Some(callback_interface.register(move |img| {
                    let _ = img.image.save(&final_path);
                }));
            }
        }
        Ok(ExecutionStatus::Success)
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![Port::input::<CallbacksBlackboard<ImageCursor>>(
            "image_cursor_cb",
        )])
    }

    fn setup_inputs(
        &mut self,
        interface: &mut dyn BlackboardInputInterface,
    ) -> Result<(), NodeError> {
        self.input_image_cursor_cb =
            interface.input::<CallbacksBlackboard<ImageCursor>>("image_cursor_cb")?;
        Ok(())
    }

    fn static_type() -> NodeType {
        "image_cursor_writer".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        self.ticket = None;
        self.config.load_node_config(config)
    }

    fn reset(&mut self) {
        self.ticket = None;
    }

    fn set_directory(&mut self, directory: Option<&std::path::Path>) {
        self.directory = directory.map(|v| v.to_owned());
        self.ticket = None;
    }
}

#[cfg(feature = "betula_editor")]
mod ui_support {
    use super::*;
    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for ImageWriteCursorNode {
        fn ui_title(&self) -> String {
            "image cursor writer ✏ ".to_owned()
        }

        fn ui_config(
            &mut self,
            _ctx: &dyn UiNodeContext,
            ui: &mut egui::Ui,
            scale: f32,
        ) -> UiConfigResponse {
            let mut token_modified = false;
            let text = "path to save screenshots in, empty is dont do anything, relative if not / at begin";
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.config.path)
                    .hint_text(text)
                    .min_size(egui::vec2(100.0 * scale, 0.0)),
            );
            token_modified |= response.on_hover_text(text).changed();
            if token_modified {
                UiConfigResponse::Changed
            } else {
                UiConfigResponse::UnChanged
            }
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("consumer".to_owned()),
                UiNodeCategory::Name("image_cursor_writer".to_owned()),
            ]
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..0
        }
    }
}
