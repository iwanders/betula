use betula_common::{control::InProcessControl, create_server_thread};
use betula_core::basic::{BasicBlackboard, BasicTree};
use betula_egui::{editor::BetulaEditor, UiSupport};
use betula_enigo;

// Factory function for the ui support.
fn create_ui_support() -> UiSupport {
    let mut ui_support = UiSupport::new();
    ui_support.add_node_default::<betula_core::nodes::SequenceNode>();
    ui_support.add_node_default::<betula_core::nodes::SelectorNode>();
    ui_support.add_node_default::<betula_core::nodes::FailureNode>();
    ui_support.add_node_default::<betula_core::nodes::SuccessNode>();
    ui_support.add_node_default::<betula_core::nodes::RunningNode>();
    ui_support.add_node_default::<betula_enigo::nodes::EnigoNode>();
    ui_support.add_node_default_with_config::<betula_common::nodes::DelayNode, betula_common::nodes::DelayNodeConfig>();
    ui_support.add_node_default_with_config::<betula_common::nodes::ParallelNode, betula_common::nodes::ParallelNodeConfig>();
    ui_support.add_node_default_with_config::<betula_enigo::nodes::EnigoTokenNode, betula_enigo::nodes::EnigoTokenNodeConfig>();
    ui_support
        .tree_support_mut()
        .set_blackboard_factory(Box::new(|| {
            Box::new(betula_core::basic::BasicBlackboard::default())
        }));
    ui_support.add_node_default::<betula_common::nodes::TimeNode>();
    ui_support.add_value_default::<f64>();
    ui_support.add_value_default::<betula_enigo::EnigoBlackboard>();
    ui_support
}

fn main() -> eframe::Result<()> {
    // Create the control pipes.
    let (server, client) = InProcessControl::new();

    // Create the background runner.
    let _background_runner = create_server_thread::<BasicTree, BasicBlackboard>(
        Box::new(|| create_ui_support().into_tree_support()),
        server,
    );

    // Create the viewer
    let ui_support = create_ui_support();

    // Run the editor.
    let mut native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };
    native_options.viewport.icon = Some(std::sync::Arc::new(betula_egui::betula_icon()));

    eframe::run_native(
        "Betula Interface",
        native_options,
        Box::new(|cx| Box::new(BetulaEditor::new(Box::new(client), ui_support, cx))),
    )
}
