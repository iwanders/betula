use eframe::egui;
use betula_egui::TreeView;

fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native("My egui App", native_options, Box::new(|cc| Box::new(MyEguiApp::new(cc))));
}

#[derive(Default)]
struct MyEguiApp {
    zoom: TreeView,
}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Self::default()
    }
}

impl eframe::App for MyEguiApp {
   fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
       egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hello World!");
            ui.label(
                "Pan, zoom in, and zoom out with scrolling (see the plot demo for more instructions). \
                       Double click on the background to reset.",
            );
            ui.vertical_centered(|ui| {
                // ui.add(crate::egui_github_link_file!());
            });
            ui.separator();
            self.zoom.ui(ui);
            ui.allocate_space(ui.available_size()); // put this LAST in your panel/window code
       });
   }
}
