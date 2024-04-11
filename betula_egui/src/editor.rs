use eframe::{App, CreationContext};

use crate::{BetulaViewer, BetulaViewerNode};
use betula_common::{control::TreeServer, TreeSupport};
use betula_core::BetulaError;
use egui_snarl::{ui::SnarlStyle, Snarl};

use std::sync::mpsc::{channel, Receiver, Sender};

// Save / load through https://github.com/woelper/egui_pick_file and https://github.com/emilk/egui/issues/270

pub struct BetulaEditor {
    snarl: Snarl<BetulaViewerNode>,
    style: SnarlStyle,
    viewer: BetulaViewer,
    text_channel: (Sender<String>, Receiver<String>),
}

impl BetulaEditor {
    pub fn new(viewer: BetulaViewer, _cx: &CreationContext) -> Self {
        let snarl = Snarl::<BetulaViewerNode>::new();

        let mut style = SnarlStyle::new();
        style.simple_wire = true;

        BetulaEditor {
            viewer,
            snarl,
            style,
            text_channel: channel(),
        }
    }
}

impl App for BetulaEditor {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let r = self.viewer.service(&mut self.snarl);
        if r.is_err() {
            println!("Error servicing: {:?}", r.err());
        }

        if let Ok(text) = self.text_channel.1.try_recv() {
            println!("text: {text:?}");
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                {
                    ui.menu_button("File", |ui| {
                        if ui.button("📂 Open").clicked() {
                            let sender = self.text_channel.0.clone();
                            let task = rfd::AsyncFileDialog::new().pick_file();
                            // Context is wrapped in an Arc so it's cheap to clone as per:
                            // > Context is cheap to clone, and any clones refers to the same mutable data (Context uses refcounting internally).
                            // Taken from https://docs.rs/egui/0.24.1/egui/struct.Context.html
                            let ctx = ui.ctx().clone();
                            execute(async move {
                                let file = task.await;
                                if let Some(file) = file {
                                    let text = file.read().await;
                                    let _ = sender.send(String::from_utf8_lossy(&text).to_string());
                                    ctx.request_repaint();
                                }
                            });
                        }

                        if ui.button("💾 Save").clicked() {
                            let task = rfd::AsyncFileDialog::new().save_file();
                            let contents = "kldsjflkdsjfldsf";
                            execute(async move {
                                let file = task.await;
                                if let Some(file) = file {
                                    _ = file.write(contents.as_bytes()).await;
                                }
                            });
                        }
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close)
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_dark_light_mode_switch(ui);
            });
        });

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

/// Function to create the tree support in the background server thread.
pub type TreeSupportCreator = Box<dyn Fn() -> TreeSupport + Send>;

/// Function to run a Tree and TreeServer in the background.
pub fn create_server_thread<T: betula_core::Tree, B: betula_core::Blackboard + 'static>(
    tree_support: TreeSupportCreator,
    server: impl TreeServer + std::marker::Send + 'static,
) -> std::thread::JoinHandle<Result<(), BetulaError>> {
    std::thread::spawn(move || -> Result<(), betula_core::BetulaError> {
        use betula_common::control::CommandResult;
        use betula_common::control::{InteractionCommand, InteractionEvent};

        let mut tree = T::new();
        let tree_support = tree_support();

        let mut run_roots: bool = true;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(10));
            let received = server.get_command()?;

            if let Some(command) = received {
                println!("    Executing {command:?}");
                if let InteractionCommand::RunSettings(run_settings) = &command {
                    if let Some(new_value) = run_settings.run_roots {
                        println!("Setting run roots to: {new_value}");
                        run_roots = new_value;
                    }
                }
                let r = command.execute(&tree_support, &mut tree);
                match r {
                    Ok(v) => {
                        for event in v {
                            server.send_event(event)?;
                        }
                    }
                    Err(e) => {
                        server.send_event(InteractionEvent::CommandResult(CommandResult {
                            command: command,
                            error: Some(format!("{e:?}")),
                        }))?;
                    }
                }
            }

            if run_roots {
                let roots = tree.roots();
                for r in &roots {
                    match tree.execute(*r) {
                        Ok(_v) => {
                            // println!("Success running {r:?}: {v:?}");
                        }
                        Err(_e) => {
                            // println!("Failed running {r:?}: {e:?}");
                        }
                    }
                }

                // Lets just dump all the blackboard state every cycle.
                if !roots.is_empty() {
                    for blackboard_id in tree.blackboards() {
                        server.send_event(InteractionEvent::BlackboardInformation(
                            InteractionCommand::blackboard_information(
                                &tree_support,
                                blackboard_id,
                                &tree,
                            )?,
                        ))?;
                    }
                }
            }
        }
    })
}
