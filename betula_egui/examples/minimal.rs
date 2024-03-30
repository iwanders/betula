use eframe::{App, CreationContext};

use betula_common::control::InProcessControl;
use betula_egui::{BetulaViewer, BetulaViewerNode};
use egui_snarl::{ui::SnarlStyle, Snarl};

pub struct DemoApp {
    snarl: Snarl<BetulaViewerNode>,
    style: SnarlStyle,
    viewer: BetulaViewer,
}

impl DemoApp {
    pub fn new(viewer: BetulaViewer, cx: &CreationContext) -> Self {
        let snarl = Snarl::<BetulaViewerNode>::new();

        let style = SnarlStyle::new();

        DemoApp {
            viewer,
            snarl,
            style,
        }
    }
}

impl App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                {
                    ui.menu_button("File", |ui| {
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

    fn save(&mut self, storage: &mut dyn eframe::Storage) {}
}

fn main() -> eframe::Result<()> {
    let (server, client) = InProcessControl::new();

    let viewer = BetulaViewer::new(Box::new(client));

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };

    eframe::run_native(
        "egui-snarl demo",
        native_options,
        Box::new(|cx| Box::new(DemoApp::new(viewer, cx))),
    )
}
