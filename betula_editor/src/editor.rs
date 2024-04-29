use eframe::{App, CreationContext};

use crate::{BetulaViewer, BetulaViewerNode, UiSupport};
use betula_common::{
    control::{InProcessControl, InteractionCommand, TreeClient, TreeServer},
    tree_support::TreeConfig,
};
use betula_core::BetulaError;
use egui_snarl::{ui::SnarlStyle, Snarl};
use serde::{Deserialize, Serialize};

use std::sync::mpsc::{channel, Receiver, Sender};

// Save / load through https://github.com/woelper/egui_pick_file and https://github.com/emilk/egui/issues/270
// Oh; https://github.com/emilk/egui/tree/master/examples/file_dialog

type SerializableHolder = serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct EditorState {
    snarl_state: SerializableHolder,
    run_state: RunState,
    color_node_status: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct EditorConfig {
    editor: EditorState,
    tree: TreeConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Milliseconds(pub u64);

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
struct RunState {
    roots: bool,
    interval: Milliseconds,
}
impl RunState {
    pub fn save(&self) -> Self {
        let mut new = *self;
        new.roots = false;
        new
    }
    pub fn command(&self) -> InteractionCommand {
        InteractionCommand::RunSettings(betula_common::control::RunSettings {
            roots: Some(self.roots),
            interval: Some(std::time::Duration::from_millis(self.interval.0)),
            ..Default::default()
        })
    }
}

impl Default for RunState {
    fn default() -> Self {
        Self {
            roots: false,
            interval: Milliseconds(50),
        }
    }
}

pub struct BetulaEditor {
    snarl: Snarl<BetulaViewerNode>,
    style: SnarlStyle,
    viewer: BetulaViewer,
    tree_config_channel: (Sender<EditorConfig>, Receiver<EditorConfig>),

    /// Client to interact with the server.
    client: Box<dyn TreeClient>,

    viewer_server: Box<dyn TreeServer>,

    pending_snarl: Option<Snarl<BetulaViewerNode>>,

    run_state: RunState,
}

impl BetulaEditor {
    pub fn new(client: Box<dyn TreeClient>, ui_support: UiSupport, _cx: &CreationContext) -> Self {
        let snarl = Snarl::<BetulaViewerNode>::new();

        let (viewer_server, viewer_client) = InProcessControl::new();

        let viewer = BetulaViewer::new(Box::new(viewer_client), ui_support);

        let mut style = SnarlStyle::new();
        style.simple_wire = true;
        // style.collapsible = false;

        BetulaEditor {
            viewer,
            snarl,
            pending_snarl: None,
            style,
            tree_config_channel: channel(),
            client,
            viewer_server: Box::new(viewer_server),
            run_state: Default::default(),
        }
    }
    pub fn client(&self) -> &dyn TreeClient {
        &*self.client
    }

    fn request_tree_config(&mut self) -> Result<(), BetulaError> {
        let cmd = InteractionCommand::request_tree_config();
        self.client.send_command(cmd)
    }

    fn send_tree_config(&mut self, config: TreeConfig) -> Result<(), BetulaError> {
        let cmd = InteractionCommand::load_tree_config(config);
        self.client.send_command(cmd)
    }

    fn send_run_settings(&self) -> Result<(), BetulaError> {
        let cmd = self.run_state.command();
        self.client.send_command(cmd)
    }
    fn send_run_roots(&self) -> Result<(), BetulaError> {
        let cmd = InteractionCommand::run_specific(&self.viewer.tree_roots());
        self.client.send_command(cmd)
    }

    fn save_tree_config(&mut self, tree: TreeConfig) -> Result<(), BetulaError> {
        let task = rfd::AsyncFileDialog::new()
            .set_file_name("tree.json")
            .add_filter("json", &["json"])
            .save_file();

        let editor = EditorState {
            snarl_state: serde_json::to_value(&self.snarl)?,
            run_state: self.run_state.save(),
            color_node_status: self.viewer.color_node_status(),
        };
        let editor_config = EditorConfig { tree, editor };
        let editor_config = serde_json::to_string_pretty(&editor_config)?;

        let contents = editor_config;
        execute(async move {
            let file = task.await;
            if let Some(file) = file {
                let r = file.write(contents.as_bytes()).await;
                if let Err(e) = r {
                    println!("Failed to save {e:?}");
                }
            }
        });
        Ok(())
    }

    fn load_editor_config(content: &[u8]) -> Result<EditorConfig, BetulaError> {
        let config: EditorConfig = serde_json::de::from_slice(content)?;
        Ok(config)
    }
    fn load_editor_config_dialog(&self) {
        let sender = self.tree_config_channel.0.clone();
        let task = rfd::AsyncFileDialog::new()
            .set_title("Open a tree")
            .pick_file();
        execute(async move {
            let file = task.await;
            if let Some(file) = file {
                let text = file.read().await;
                let config = Self::load_editor_config(&text);
                if let Ok(config) = config {
                    let _ = sender.send(config);
                } else {
                    println!("Failed to load config: {config:?}");
                }
                // ctx.request_repaint();
            }
        });
    }
    fn load_editor_state(
        &mut self,
        editor_state: EditorState,
    ) -> Result<Snarl<BetulaViewerNode>, BetulaError> {
        let snarl: Snarl<BetulaViewerNode> = serde_json::from_value(editor_state.snarl_state)?;
        Ok(snarl)
    }

    fn service(&mut self, ctx: &egui::Context) -> Result<(), BetulaError> {
        let _ = ctx;

        if let Ok(config) = self.tree_config_channel.1.try_recv() {
            // Pause the execution!
            self.run_state = config.editor.run_state;
            self.run_state.roots = false;
            self.viewer
                .set_color_node_status(config.editor.color_node_status);
            self.send_run_settings()?;
            self.pending_snarl = Some(self.load_editor_state(config.editor)?);
            self.send_tree_config(config.tree)?;
        }

        loop {
            let viewer_cmd_received = self.viewer_server.get_command()?;
            let backend_event_received = self.client.get_event()?;

            if viewer_cmd_received.is_none() && backend_event_received.is_none() {
                break;
            }

            if let Some(viewer_cmd) = viewer_cmd_received {
                // Just pass to the backend.
                self.client.send_command(viewer_cmd)?;
            }
            if let Some(backend_event) = backend_event_received {
                // println!("event: {backend_event:?}");
                use betula_common::control::InteractionEvent;
                use betula_common::control::InteractionEvent::{CommandResult, TreeConfig};
                let c = match backend_event {
                    CommandResult(ref c) => match c.command {
                        InteractionCommand::RequestTreeConfig => {
                            println!("failed to get tree config");
                            None
                        }
                        InteractionCommand::RunSettings(ref e) => {
                            if let Some(new_value) = &e.roots {
                                self.run_state.roots = *new_value;
                            }
                            if let Some(interval) = &e.interval {
                                self.run_state.interval = Milliseconds(interval.as_millis() as u64);
                            }
                            None
                        }
                        _ => Some(backend_event),
                    },
                    TreeConfig(v) => {
                        println!("Got config: {v:?}");
                        self.save_tree_config(v)?;
                        None
                    }
                    InteractionEvent::TreeState(state) => {
                        if let Some(pending_snarl) = self.pending_snarl.take() {
                            self.viewer
                                .set_tree_state(state, &mut self.snarl, pending_snarl)?;
                        }
                        None
                    }
                    _ => Some(backend_event),
                };
                // Just pass to the backend.
                if let Some(viewer_event) = c {
                    self.viewer_server.send_event(viewer_event)?;
                }
            }
        }

        let r = self.viewer.service(&mut self.snarl);
        if r.is_err() {
            println!("Error servicing viewer: {:?}", r.err());
        }
        Ok(())
    }

    pub fn ui_top_panel(&mut self, ctx: &egui::Context) -> Result<(), BetulaError> {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                {
                    ui.menu_button("File", |ui| {
                        if ui.button("📂 Open").clicked() {
                            self.load_editor_config_dialog();
                            ui.close_menu();
                        }

                        if ui.button("💾 Save as").clicked() {
                            let r = self.request_tree_config();
                            if let Err(e) = r {
                                println!("Failed to request config: {e:?}");
                            }
                            ui.close_menu();
                        }
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close)
                        }
                    });
                    ui.add_space(16.0);
                }
                ui.separator();
                let symbol = if self.run_state.roots { "⏸" } else { "▶" };
                let mut state_changed = false;
                if ui.button(symbol).clicked() {
                    // ⏸
                    self.run_state.roots = !self.run_state.roots;
                    state_changed = true;
                }
                let r = ui.add(
                    egui::DragValue::new(&mut self.run_state.interval.0)
                        .clamp_range(20..=10000)
                        .suffix("ms")
                        .update_while_editing(false),
                );
                if r.changed() {
                    state_changed = true;
                }
                if state_changed {
                    if let Err(e) = self.send_run_settings() {
                        println!("Error servicing: {e:?}");
                    }
                }

                if ui.button("⏭").clicked() {
                    if let Err(e) = self.send_run_roots() {
                        println!("Error servicing: {e:?}");
                    }
                }
                // 📥
                ui.separator();

                let mut node_color_status = self.viewer.color_node_status();
                if ui.checkbox(&mut node_color_status, "Color").changed() {
                    self.viewer.set_color_node_status(node_color_status);
                    if !node_color_status {
                        self.viewer.clear_execution_results(&mut self.snarl);
                    }
                }

                ui.separator();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::widgets::global_dark_light_mode_switch(ui);
                });
            });
        });
        Ok(())
    }
}

impl App for BetulaEditor {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // egui_extras::install_image_loaders(ctx);
        let r = self.service(ctx);
        if r.is_err() {
            println!("Error servicing: {:?}", r.err());
        }

        let r = self.ui_top_panel(ctx);
        if r.is_err() {
            println!("Error top pannel: {:?}", r.err());
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            self.snarl
                .show(&mut self.viewer, &self.style, egui::Id::new("snarl"), ui);
        });
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}
}

#[cfg(not(target_arch = "wasm32"))]
fn execute<F: std::future::Future<Output = ()> + Send + 'static>(f: F) {
    // this is stupid... use any executor of your choice instead
    std::thread::spawn(move || smol::block_on(f));
}

#[cfg(target_arch = "wasm32")]
fn execute<F: std::future::Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}
