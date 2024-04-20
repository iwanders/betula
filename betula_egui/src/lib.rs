/*! A viewer for Betula Behaviour trees.
*/

pub mod nodes;

mod ui;
pub use ui::{UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext, UiSupport, UiValue};

mod viewer;
pub use viewer::{BetulaViewer, BetulaViewerNode, ViewerNode};

pub mod editor;

pub fn betula_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(&include_bytes!("../../media/icon.png")[..]).unwrap()
}

pub mod widgets;
pub use egui;

/// Register core and common nodes to the ui support.
pub fn add_ui_support(ui_support: &mut crate::UiSupport) {
    // ui_support.add_node_default_with_config::<crate::nodes::EnigoInstanceNode, crate::nodes::EnigoInstanceNodeConfig>();
    // ui_support.add_node_default_with_config::<crate::nodes::EnigoNode, crate::nodes::EnigoNodeConfig>();
    // ui_support.add_value_default_named::<crate::EnigoBlackboard>("Enigo");
    ui_support.add_node_default::<betula_core::nodes::SequenceNode>();
    ui_support.add_node_default::<betula_core::nodes::SelectorNode>();
    ui_support.add_node_default::<betula_core::nodes::FailureNode>();
    ui_support.add_node_default::<betula_core::nodes::SuccessNode>();
    ui_support.add_node_default::<betula_core::nodes::RunningNode>();
    ui_support.add_node_default_with_config::<betula_common::nodes::DelayNode, betula_common::nodes::DelayNodeConfig>();
    ui_support.add_node_default_with_config::<betula_common::nodes::ParallelNode, betula_common::nodes::ParallelNodeConfig>();
    ui_support
        .tree_support_mut()
        .set_blackboard_factory(Box::new(|| {
            Box::new(betula_core::basic::BasicBlackboard::default())
        }));
    ui_support.add_node_default::<betula_common::nodes::TimeNode>();
    ui_support.add_value_default::<f64>();
}
