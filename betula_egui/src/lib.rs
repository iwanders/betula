/*! A viewer for Betula Behaviour trees.
*/

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
pub mod core;

/// Register core nodes to the ui support.
pub fn add_ui_support(ui_support: &mut UiSupport) {
    ui_support.add_node_default::<betula_core::nodes::SequenceNode>();
    ui_support.add_node_default::<betula_core::nodes::SelectorNode>();
    ui_support.add_node_default::<betula_core::nodes::FailureNode>();
    ui_support.add_node_default::<betula_core::nodes::SuccessNode>();
    ui_support.add_node_default::<betula_core::nodes::RunningNode>();
}
