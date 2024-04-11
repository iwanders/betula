use betula_common::control::InProcessControl;
use betula_core::basic::{BasicBlackboard, BasicTree};
use betula_egui::{
    editor::{create_server_thread, BetulaEditor},
    BetulaViewer, UiSupport,
};

// Factory function for the ui support.
fn create_ui_support() -> UiSupport {
    let mut ui_support = UiSupport::new();
    ui_support.add_node_default::<betula_core::nodes::SequenceNode>();
    ui_support.add_node_default::<betula_core::nodes::SelectorNode>();
    ui_support.add_node_default::<betula_core::nodes::FailureNode>();
    ui_support.add_node_default::<betula_core::nodes::SuccessNode>();
    ui_support.add_node_default_with_config::<betula_common::nodes::DelayNode, betula_common::nodes::DelayNodeConfig>();
    ui_support
        .tree_support_mut()
        .set_blackboard_factory(Box::new(|| {
            Box::new(betula_core::basic::BasicBlackboard::default())
        }));
    ui_support.add_node_default::<betula_common::nodes::TimeNode>();
    ui_support.add_value_default::<f64>();
    ui_support
}

fn main() -> eframe::Result<()> {
    // Create the control pipes.
    let (server, client) = InProcessControl::new();

    // Create the background runner.
    let _background_runner = create_server_thread::<BasicTree, BasicBlackboard, _>(
        Box::new(|| create_ui_support().into_tree_support()),
        server,
    );

    // Create the viewer
    let ui_support = create_ui_support();
    let viewer = BetulaViewer::new(Box::new(client), ui_support);

    // Run the viewer.
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Betula Interface",
        native_options,
        Box::new(|cx| Box::new(BetulaEditor::new(viewer, cx))),
    )
}
