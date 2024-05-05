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

pub trait MenuEntry: std::hash::Hash + std::cmp::PartialEq {
    fn label(&self) -> &str;
    fn hover(&self) -> Option<&str>;
}

#[derive(Clone, Hash, Debug, Ord, Eq, PartialEq, PartialOrd)]
pub struct MenuEntryData {
    pub label: String,
    pub hover: Option<String>,
}
impl MenuEntry for MenuEntryData {
    fn label(&self) -> &str {
        self.label.as_ref()
    }
    fn hover(&self) -> Option<&str> {
        self.hover.as_ref().map(|v| v.as_str())
    }
}

pub type UiMenuTree<K, T> = std::collections::BTreeMap<K, UiMenuNode<K, T>>;
pub enum UiMenuNode<K: MenuEntry, T> {
    Value(T),
    SubElements(UiMenuTree<K, T>),
}
impl<K: MenuEntry, T> UiMenuNode<K, T> {
    pub fn sub_elements(&mut self) -> &mut UiMenuTree<K, T> {
        if let UiMenuNode::<K, T>::SubElements(z) = self {
            return z;
        }
        panic!("sub elements called on non subelement enum");
    }
}

pub fn menu_node_recurser<K: MenuEntry, T: Clone>(
    tree: &UiMenuTree<K, T>,
    ui: &mut egui::Ui,
) -> Option<T> {
    for (info, element) in tree.iter() {
        match element {
            UiMenuNode::<K, T>::Value(ref v) => {
                let mut button = ui.button(info.label());
                if let Some(s) = info.hover() {
                    button = button.on_hover_text(s);
                }
                if button.clicked() {
                    ui.close_menu();
                    return Some(v.clone());
                }
            }
            UiMenuNode::<K, T>::SubElements(ref subtree) => {
                let z = ui.menu_button(info.label(), |ui| menu_node_recurser(subtree, ui));
                if let Some(returned_node_type) = z.inner.flatten() {
                    return Some(returned_node_type);
                }
            }
        }
    }

    None
}
