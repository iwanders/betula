pub mod nodes;

/// Register standard nodes to the ui support.
#[cfg(feature = "betula_egui")]
pub fn add_ui_support(ui_support: &mut betula_egui::UiSupport) {
    ui_support.add_node_default::<nodes::SequenceNode>();
    ui_support.add_node_default::<nodes::SelectorNode>();
    ui_support.add_node_default::<nodes::FailureNode>();
    ui_support.add_node_default::<nodes::SuccessNode>();
    ui_support.add_node_default::<nodes::RunningNode>();

    ui_support.add_node_default_with_config::<nodes::DelayNode, nodes::DelayNodeConfig>();
    ui_support.add_node_default_with_config::<nodes::ParallelNode, nodes::ParallelNodeConfig>();
    ui_support.add_node_default::<nodes::TimeNode>();
    ui_support.add_value_default::<f64>();
}