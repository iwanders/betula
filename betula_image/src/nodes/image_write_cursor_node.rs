use crate::ImageCursor;
use betula_common::callback::{CallbacksBlackboard, Ticket};
use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicI64;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ImageWriteCursorNodeConfig {
    /// The path to write the files to.
    path: String,

    /// The number of frames to set the counter to for each execution, it NOT an increment, it is a
    /// set. If the counter reaches zero, no further frames are saved.
    frames_to_save: i64,
}
impl IsNodeConfig for ImageWriteCursorNodeConfig {}

#[derive(Default)]
pub struct ImageWriteCursorNode {
    /// Input for the callback object.
    input_image_cursor_cb: Input<CallbacksBlackboard<ImageCursor>>,

    /// The directory used to create the final path.
    directory: Option<std::path::PathBuf>,

    /// The config that holds the relative path.
    pub config: ImageWriteCursorNodeConfig,

    /// The ticket to hang on to to keep the callback active.
    ticket: Option<Ticket<ImageCursor>>,

    /// Thread pool to perform saving of the images.
    pool: Arc<threadpool::ThreadPool>,

    /// The amount of frames that can still be captured.
    frames_to_save: Arc<AtomicI64>,
}

impl std::fmt::Debug for ImageWriteCursorNode {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "ImageWriteCursorNode")
    }
}

impl Node for ImageWriteCursorNode {
    fn execute(&mut self, _ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        self.frames_to_save
            .store(self.config.frames_to_save, SeqCst);

        if self.ticket.is_none() {
            let callback_value = self.input_image_cursor_cb.get()?;
            let callback_interface = callback_value
                .callbacks()
                .ok_or(format!("callbacks not populated yet"))?;
            if self.config.path.is_empty() {
                self.ticket = Some(callback_interface.register(|_| {}));
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

                let pool = Arc::clone(&self.pool);
                let fp = final_path.clone();
                let string_path = fp
                    .into_os_string()
                    .into_string()
                    .map_err(|e| format!("failed convering path: {e:?}"))?;

                let string_path = string_path.trim_end_matches(".png").to_owned();
                let counter = Arc::clone(&self.frames_to_save);
                self.ticket = Some(callback_interface.register(move |img| {
                    let value = counter.load(SeqCst);
                    if value >= 0 {
                        counter.fetch_sub(1, SeqCst);
                    } else {
                        return;
                    }

                    let ctr = format!("{:0>6}", img.counter);
                    let path = string_path.replace("{c}", &ctr);
                    let t = format!("{}", (img.time * 1000.0).floor() as u64);
                    let path = path.replace("{t}", &t);

                    // Don't block the main callback queue, so dispatch to the pool.
                    pool.execute(move || {
                        let mut png_path = path.clone();
                        png_path.push_str(".png");
                        let _ = img.image.save(&png_path);

                        let mut json_path = path.clone();
                        json_path.push_str(".json");
                        use std::fs::File;
                        use std::io::BufWriter;
                        if let Ok(file) = File::create(json_path) {
                            let mut writer = BufWriter::new(file);
                            let _ = serde_json::to_writer(&mut writer, &img);
                        }
                    });
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

    fn reset(&mut self) {}

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

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                let text = "path to save screenshots in, empty is dont do anything, added to directory if not absolute, extension is always png.";
                let text2 = "{c} gets replaced with the frame counter and some zeros 000001";
                let text3 = "{t} gets replaced with the unix timestamp in msec";
                ui.label("Path");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.config.path)
                        .hint_text(text)
                        .min_size(egui::vec2(100.0 * scale, 0.0)),
                );
                token_modified |= response.on_hover_text(text).on_hover_text(text2).on_hover_text(text3).changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Save Count");
                    let r = ui.add(
                        egui::DragValue::new(&mut self.config.frames_to_save)
                            .clamp_range(0..=600)
                            .update_while_editing(false),
                    );
                    token_modified |= r.on_hover_text("set the frame save counter to this each execution").changed();
                });
            });

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
