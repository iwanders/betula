use betula_core::basic::BasicTree;
use betula_core::{nodes, NodeId, Uuid};
use betula_egui::TreeView;
use eframe::egui;

fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "My egui App",
        native_options,
        Box::new(|cc| Box::new(MyEguiApp::new(cc))),
    );
}

struct MyEguiApp {
    view: TreeView,
    bt: BasicTree,
}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        use betula_core::nodes;
        use betula_core::prelude::*;
        let mut bt = BasicTree::new();
        let root = bt
            .add_node_boxed(NodeId(Uuid::new_v4()), Box::new(nodes::Selector {}))
            .unwrap();
        let f1 = bt
            .add_node_boxed(NodeId(Uuid::new_v4()), Box::new(nodes::Failure {}))
            .unwrap();
        let s1 = bt
            .add_node_boxed(NodeId(Uuid::new_v4()), Box::new(nodes::Success {}))
            .unwrap();
        bt.add_relation(root, 0, f1);
        bt.add_relation(root, 1, s1);
        // let res = bt.run(root);
        // assert_eq!(res.ok(), Some(Status::Success));
        let view = TreeView::default();
        MyEguiApp { view, bt }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.view.update(&self.bt);
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
            self.view.ui(ui);
            ui.allocate_space(ui.available_size()); // put this LAST in your panel/window code
       });
    }
}
