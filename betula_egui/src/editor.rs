use eframe::{App, CreationContext};

use betula_core::{BetulaError};
use betula_common::{control::{TreeServer}, TreeSupport};
use crate::{BetulaViewer, BetulaViewerNode};
use egui_snarl::{ui::SnarlStyle, Snarl};


// Save / load example from https://github.com/c-git/egui_file_picker_poll_promise/tree/main
// Through https://github.com/woelper/egui_pick_file and https://github.com/emilk/egui/issues/270

type SaveLoadPromise = Option<poll_promise::Promise<Option<String>>>;
pub struct BetulaEditor {
    snarl: Snarl<BetulaViewerNode>,
    style: SnarlStyle,
    viewer: BetulaViewer,

    save_load_promise: SaveLoadPromise,
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
            save_load_promise: None,
        }
    }
}


impl App for BetulaEditor {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let r = self.viewer.service(&mut self.snarl);
        if r.is_err() {
            println!("Error servicing: {:?}", r.err());
        }

        // assign sample text once it comes in
        if let Some(promise) = &self.save_load_promise {
            if promise.ready().is_some() {
                // Clear promise and take the value out
                // Doesn't matter for string as we can just clone it but depending on the type you have
                // you may not be able to easily clone it and would prefer get the owned value
                let mut temp = None;
                std::mem::swap(&mut temp, &mut self.save_load_promise);

                let owned_promise = temp.expect("we got here because it was some");
                let inner_option = owned_promise.block_and_take(); // This should be fine because we know it's ready

                if let Some(text) = inner_option {
                    // self.sample_text = text;
                    println!("Something was inner");
                } else {
                    // User probably cancelled or it was saving but the promise completed either way
                }
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                {
                    ui.menu_button("File", |ui| {
                        if ui.button("Open").clicked() {
                            let ctx = ui.ctx().clone();
                            self.save_load_promise = execute(async move {
                                let file = rfd::AsyncFileDialog::new().pick_file().await?; // Returns None if file is None
                                let text = file.read().await;

                                // If not present screen will not refresh until next paint (comment out to test, works better with the sleep above to demonstrate)
                                ctx.request_repaint();

                                Some(String::from_utf8_lossy(&text).to_string())
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


#[cfg(target_arch = "wasm32")]
fn execute<F>(f: F) -> SaveLoadPromise
where
    F: std::future::Future<Output = Option<String>> + 'static,
{
    Some(poll_promise::Promise::spawn_local(f))
}

#[cfg(not(target_arch = "wasm32"))]
fn execute<F>(f: F) -> SaveLoadPromise
where
    F: std::future::Future<Output = Option<String>> + std::marker::Send + 'static,
{
    Some(poll_promise::Promise::spawn_async(f))
}


/// Function to create the tree support in the background server thread.
pub type TreeSupportCreator = Box<dyn Fn() -> TreeSupport + Send>;

/// Function to run a Tree and TreeServer in the background.
pub fn create_server_thread<T: betula_core::Tree, B:betula_core::Blackboard + 'static>(tree_support: TreeSupportCreator, server: impl TreeServer + std::marker::Send + 'static) -> std::thread::JoinHandle<Result<(), BetulaError>> {

    std::thread::spawn(move || -> Result<(), betula_core::BetulaError> {

        use betula_common::control::CommandResult;
        use betula_common::control::{InteractionEvent, InteractionCommand};


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
                        server.send_event(InteractionEvent::BlackboardInformation(InteractionCommand::blackboard_information(&tree_support, blackboard_id, &tree)?))?;
                    }
                }
            }
        }
    })
}
