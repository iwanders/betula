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
    pub fn new(viewer: BetulaViewer, _cx: &CreationContext) -> Self {
        let snarl = Snarl::<BetulaViewerNode>::new();

        let mut style = SnarlStyle::new();
        style.simple_wire = true;

        DemoApp {
            viewer,
            snarl,
            style,
        }
    }
}

impl App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let r = self.viewer.service(&mut self.snarl);
        if r.is_err() {
            println!("Error servicing: {:?}", r.err());
        }
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

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}
}

fn main() -> eframe::Result<()> {
    let (server, client) = InProcessControl::new();

    let _server_thing = std::thread::spawn(move || -> Result<(), betula_core::BetulaError> {
        use betula_common::control::TreeServer;
        use betula_common::TreeSupport;
        use betula_core::basic::BasicTree;

        use betula_common::control::CommandResult;
        use betula_common::control::InteractionEvent;

        let mut tree = BasicTree::new();
        let mut tree_support = TreeSupport::new();
        tree_support.add_node_default::<betula_core::nodes::SequenceNode>();
        tree_support.add_node_default::<betula_core::nodes::SelectorNode>();
        tree_support.add_node_default::<betula_core::nodes::FailureNode>();
        tree_support.add_node_default::<betula_core::nodes::SuccessNode>();
        tree_support.add_node_default_with_config::<betula_common::nodes::DelayNode, betula_common::nodes::DelayNodeConfig>(
            );
        tree_support.add_node_default::<betula_common::nodes::TimeNode>();
        tree_support.set_blackboard_factory(Box::new(|| {
            Box::new(betula_core::basic::BasicBlackboard::default())
        }));
        tree_support.add_value_default::<f64>();

        loop {
            std::thread::sleep(std::time::Duration::from_millis(10));
            let received = server.get_command()?;

            if let Some(command) = received {
                println!("    Executing {command:?}");
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
        }
    });

    let mut ui_support = betula_egui::UiSupport::new();
    // ui_support.add_node_default::<betula_core::nodes::SequenceNode>();
    // ui_support.add_node_default::<betula_core::nodes::SelectorNode>();
    // ui_support.add_node_default::<betula_core::nodes::FailureNode>();
    // ui_support.add_node_default::<betula_core::nodes::SuccessNode>();
    ui_support.add_node_default_with_config::<betula_common::nodes::DelayNode, betula_common::nodes::DelayNodeConfig>();
    // ui_support.add_node_default_with_config::<betula_common::nodes::DelayNode>();
    ui_support.set_blackboard_factory(Box::new(|| {
        Box::new(betula_core::basic::BasicBlackboard::default())
    }));
    ui_support.add_node_default::<betula_common::nodes::TimeNode>();
    let viewer = BetulaViewer::new(Box::new(client), ui_support);

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
