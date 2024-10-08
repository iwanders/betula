pub mod nodes;

/// Register standard nodes to the ui support.
#[cfg(feature = "betula_editor")]
pub fn add_ui_support(ui_support: &mut betula_editor::UiSupport) {
    ui_support.add_node_default_with_config::<nodes::SequenceNode, nodes::SequenceNodeConfig>();
    ui_support.add_node_default_with_config::<nodes::SelectorNode, nodes::SelectorNodeConfig>();
    ui_support.add_node_default::<nodes::FailureNode>();
    ui_support.add_node_default::<nodes::SuccessNode>();
    ui_support.add_node_default::<nodes::RunningNode>();
    ui_support.add_node_default::<nodes::BlockResetNode>();
    ui_support.add_node_default::<nodes::NegateNode>();
    ui_support.add_node_default::<nodes::ForceSuccessNode>();

    ui_support.add_node_default_with_config::<nodes::DelayNode, nodes::DelayNodeConfig>();
    ui_support
        .add_node_default_with_config::<nodes::IfTimeExceedsNode, nodes::IfTimeExceedsNodeConfig>();
    ui_support.add_node_default_with_config::<nodes::ParallelNode, nodes::ParallelNodeConfig>();
    ui_support.add_node_default_with_config::<nodes::RetryNode, nodes::RetryNodeConfig>();
    ui_support.add_node_default::<nodes::TimeNode>();
    ui_support.add_node_default::<nodes::StatusWriteNode>();
    ui_support.add_node_default::<nodes::StatusReadNode>();
    ui_support.add_node_default_with_config::<nodes::IfThenElseNode, nodes::IfThenElseNodeConfig>();
    ui_support.add_node_default_with_config::<nodes::TimeSliceNode, nodes::TimeSliceNodeConfig>();
    ui_support.add_value_default::<f64>();
    ui_support.add_value_default::<i64>();
    ui_support.add_value_default_named::<String>("String");
    ui_support.add_value_default_named::<betula_core::ExecutionStatus>("status");
    ui_support.add_node_default_with_config::<nodes::IfExecutionStatusNode, nodes::IfExecutionStatusNodeConfig>();
    ui_support
        .add_node_default_with_config::<nodes::StringWriteNode, nodes::StringWriteNodeConfig>();
}
