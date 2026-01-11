use eframe::{App, CreationContext};

use crate::{BetulaViewer, BetulaViewerNode, UiSupport};
use betula_common::{
    control::{internal_server_client, InteractionCommand, TreeClient, TreeServer},
    tree_support::TreeConfig,
};
use betula_core::BetulaError;
use egui_snarl::{ui::SnarlStyle, Snarl};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

struct PathConfig {
    path: PathBuf,
    config: EditorConfig,
}

pub struct BetulaEditor {
    snarl: Snarl<BetulaViewerNode>,
    style: SnarlStyle,
    viewer: BetulaViewer,
    tree_config_load_channel: (Sender<PathConfig>, Receiver<PathConfig>),
    tree_config_save_channel: (Sender<PathBuf>, Receiver<PathBuf>),

    /// Client to interact with the server.
    client: Box<dyn TreeClient>,

    viewer_server: Box<dyn TreeServer>,

    pending_snarl: Option<Snarl<BetulaViewerNode>>,

    run_state: RunState,

    /// Path of the current project, it's dirname is used as directory.
    path: Option<PathBuf>,

    /// The next configuration is stored to this path.
    save_path: Option<PathBuf>,

    /// Whether the viewer is hidden
    viewer_hidden: bool,
}

#[derive(Debug, Default)]
pub struct EditorOptions {
    pub open_file: Option<std::path::PathBuf>,
}

impl BetulaEditor {
    pub fn new(
        client: Box<dyn TreeClient>,
        ui_support: UiSupport,
        cx: &CreationContext,
        options: &EditorOptions,
    ) -> Self {
        let snarl = Snarl::<BetulaViewerNode>::new();

        let (viewer_server, viewer_client) = internal_server_client();

        let viewer = BetulaViewer::new(Box::new(viewer_client), ui_support);

        let mut style = SnarlStyle::new();
        style.bg_pattern = Some(egui_snarl::ui::BackgroundPattern::Grid(
            egui_snarl::ui::Grid::default(),
        ));
        // style.simple_wire = true;
        // style.collapsible = false;

        // Lets just force dark mode for now, the colors are made for that.
        cx.egui_ctx.set_visuals(egui::Visuals::dark());

        let editor = BetulaEditor {
            viewer,
            snarl,
            pending_snarl: None,
            style,
            tree_config_load_channel: channel(),
            tree_config_save_channel: channel(),
            client,
            viewer_server: Box::new(viewer_server),
            run_state: Default::default(),
            path: None,
            save_path: None,
            viewer_hidden: false,
        };

        // Now that the editor exist, we can process the options.
        if let Some(path) = &options.open_file {
            editor
                .load_editor_config_file(&path)
                .expect(&format!("failed to open {path:?}"));
        }

        editor
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
    fn send_reset_nodes(&self) -> Result<(), BetulaError> {
        let cmd = InteractionCommand::reset_nodes();
        self.client.send_command(cmd)
    }

    fn send_set_directory(&self, path: Option<&std::path::Path>) -> Result<(), BetulaError> {
        let cmd = InteractionCommand::set_directory(path);
        println!("cmd: {cmd:?}");
        self.client.send_command(cmd)
    }

    fn save_tree_config(&mut self, tree: TreeConfig) -> Result<(), BetulaError> {
        // Two options, one is we have a save_path, otherwise it is a request for a prompt.
        let editor = EditorState {
            snarl_state: serde_json::to_value(&self.snarl)?,
            run_state: self.run_state.save(),
            color_node_status: self.viewer.color_node_status(),
        };
        let editor_config = EditorConfig { tree, editor };
        let editor_config = serde_json::to_string_pretty(&editor_config)?;

        let contents = editor_config;
        if let Some(destination) = self.save_path.take() {
            if let Err(e) = std::fs::write(destination.clone(), contents.as_bytes()) {
                println!("Failed to write to {destination:?}, error: {e:?}");
            } else {
                self.set_project_path(Some(destination.clone()));
                let dir = destination.parent();
                self.send_set_directory(dir)?;
            }
        } else {
            let send_channel = self.tree_config_save_channel.0.clone();
            let task = rfd::AsyncFileDialog::new()
                .set_file_name("tree.json")
                .add_filter("json", &["json"])
                .save_file();
            execute(async move {
                let file = task.await;
                if let Some(file) = file {
                    let _ = send_channel.send(file.path().to_owned());
                    let r = file.write(contents.as_bytes()).await;
                    if let Err(e) = r {
                        println!("Failed to save {e:?}");
                    }
                }
            });
        }
        Ok(())
    }
    fn save_to_path(&mut self) {
        // Okay, this is a bit hairy... we just set a path for the next config to be stored to this path...
        if let Err(e) = self.request_tree_config() {
            println!("Failed to request tree config: {e:?}");
        }
        self.save_path = self.path.clone();
    }

    fn set_project_path(&mut self, path: Option<PathBuf>) {
        // Actually store the path.
        self.path = path.clone();
        // Set the directory.
        if let Some(path) = path {
            let dir = path.parent();
            let dir = dir.map(|v| v.to_owned());
            self.viewer.set_directory(dir.clone());
        }
    }

    fn load_editor_config(content: &[u8]) -> Result<EditorConfig, BetulaError> {
        let config: EditorConfig = serde_json::de::from_slice(content)?;
        Ok(config)
    }

    pub fn load_editor_config_file(&self, path: &std::path::Path) -> Result<(), BetulaError> {
        let sender = self.tree_config_load_channel.0.clone();
        let content = std::fs::read(path)?;
        let config: EditorConfig = serde_json::de::from_slice(&content)?;

        let path = path.to_owned();
        let pathconfig = PathConfig { config, path };
        let _ = sender.send(pathconfig);
        Ok(())
    }

    fn load_editor_config_dialog(&self) {
        let sender = self.tree_config_load_channel.0.clone();
        let task = rfd::AsyncFileDialog::new()
            .set_title("Open a tree")
            .pick_file();
        execute(async move {
            let file = task.await;
            if let Some(file) = file {
                let text = file.read().await;
                let config = Self::load_editor_config(&text);
                if let Ok(config) = config {
                    let path = file.path().to_owned();
                    let pathconfig = PathConfig { config, path };
                    let _ = sender.send(pathconfig);
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

        if let Ok(path_config) = self.tree_config_load_channel.1.try_recv() {
            // This is the new active path
            let dir_path = path_config.path.clone();
            self.set_project_path(Some(path_config.path));

            // Pause the execution!
            self.run_state = path_config.config.editor.run_state;
            self.run_state.roots = false;
            self.viewer
                .set_color_node_status(path_config.config.editor.color_node_status);
            self.send_run_settings()?;
            self.pending_snarl = Some(self.load_editor_state(path_config.config.editor)?);
            self.send_tree_config(path_config.config.tree)?;

            // Also call set directory for this new directory.
            let dir = dir_path.parent();
            self.send_set_directory(dir)?;
        }
        if let Ok(new_path) = self.tree_config_save_channel.1.try_recv() {
            // Save as happened, set the new path and use it as the new directory.
            self.set_project_path(Some(new_path.clone()));
            let dir = new_path.parent();
            self.send_set_directory(dir)?;
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
                            if let Some(e) = &c.error {
                                println!("failed to get tree config: {e:?}");
                            }
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

                        if ui
                            .add_enabled(self.path.is_some(), egui::Button::new("💾 Save"))
                            .clicked()
                        {
                            self.save_to_path();
                        }

                        if ui.button("💾 Save as...").clicked() {
                            self.save_path = None;
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
                        .clamp_range(1..=10000)
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
                ui.separator();
                if ui.button("reset nodes").clicked() {
                    if let Err(e) = self.send_reset_nodes() {
                        println!("Error servicing: {e:?}");
                    }
                }
                ui.separator();

                let mut node_color_status = self.viewer.color_node_status();
                if ui.checkbox(&mut node_color_status, "Color").changed() {
                    self.viewer.set_color_node_status(node_color_status);
                    if !node_color_status {
                        self.viewer.clear_execution_results(&mut self.snarl);
                    }
                }
                ui.separator();
                if let Some(path) = &self.path {
                    ui.label(format!("path: {:?}", path));
                    if ui.button("💾").clicked() {
                        self.save_to_path();
                    }
                } else {
                    ui.label("no path");
                }
                ui.separator();
                ui.checkbox(&mut self.viewer_hidden, "Hide Viewer");

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
        if !self.viewer_hidden {
            egui::CentralPanel::default().show(ctx, |ui| {
                self.snarl
                    .show(&mut self.viewer, &self.style, egui::Id::new("snarl"), ui);
            });
        }
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
