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
