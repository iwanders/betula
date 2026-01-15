pub trait UiMenuEntry: std::hash::Hash + std::cmp::PartialEq {
    fn label(&self) -> &str;
    fn hover(&self) -> Option<&str> {
        None
    }
}

impl UiMenuEntry for std::string::String {
    fn label(&self) -> &str {
        self.as_str()
    }
}

#[derive(Clone, Hash, Debug, Ord, Eq, PartialEq, PartialOrd)]
pub struct UiMenuEntryData {
    pub label: String,
    pub hover: Option<String>,
}
impl UiMenuEntry for UiMenuEntryData {
    fn label(&self) -> &str {
        self.label.as_ref()
    }
    fn hover(&self) -> Option<&str> {
        self.hover.as_ref().map(|v| v.as_str())
    }
}

pub type UiMenuTree<K, T> = std::collections::BTreeMap<K, UiMenuNode<K, T>>;
pub enum UiMenuNode<K: UiMenuEntry, T> {
    Value(T),
    Groups(UiMenuTree<K, T>),
    SubElements(UiMenuTree<K, T>),
}

impl<K: UiMenuEntry, T> UiMenuNode<K, T> {
    pub fn sub_elements(&mut self) -> &mut UiMenuTree<K, T> {
        if let UiMenuNode::<K, T>::SubElements(z) = self {
            return z;
        }
        panic!("sub elements called on non subelement enum");
    }
    pub fn groups(&mut self) -> &mut UiMenuTree<K, T> {
        if let UiMenuNode::<K, T>::Groups(z) = self {
            return z;
        }
        panic!("sub elements called on non subelement enum");
    }
}

pub fn menu_node_recurser<K: UiMenuEntry, T: Clone>(
    tree: &UiMenuTree<K, T>,
    ui: &mut egui::Ui,
) -> Option<T> {
    for (info, element) in tree.iter() {
        match element {
            UiMenuNode::<K, T>::Value(v) => {
                let mut button = ui.button(info.label());
                if let Some(s) = info.hover() {
                    button = button.on_hover_text(s);
                }
                if button.clicked() {
                    ui.close_menu();
                    return Some(v.clone());
                }
            }
            UiMenuNode::<K, T>::Groups(subtree) => {
                ui.label(info.label());
                if let Some(z) = menu_node_recurser(subtree, ui) {
                    return Some(z);
                }
            }
            UiMenuNode::<K, T>::SubElements(subtree) => {
                let z = ui.menu_button(info.label(), |ui| menu_node_recurser(subtree, ui));
                if let Some(returned_node_type) = z.inner.flatten() {
                    return Some(returned_node_type);
                }
            }
        }
    }

    None
}

pub fn time_drag_value_builder<F: FnOnce(egui::DragValue) -> egui::DragValue>(
    ui: &mut egui::Ui,
    value: &mut f64,
    f: F,
) -> egui::Response {
    let speed = if *value < 1.0 {
        0.001
    } else if *value < 10.0 {
        0.01
    } else {
        0.1
    };

    ui.add(f(egui::DragValue::new(value)
        .clamp_range(0.0f64..=(24.0 * 60.0 * 60.0))
        .speed(speed)
        .custom_formatter(|v, _| {
            if v < 10.0 {
                format!("{:.0} ms", v * 1000.0)
            } else if v < 60.0 {
                format!("{:.3} s", v)
            } else {
                format!("{:.0} s", v)
            }
        })
        .custom_parser(|s| {
            let parts: Vec<&str> = s.split(' ').collect();
            let value = parts[0].parse::<f64>().ok()?;
            if let Some(scale) = parts.get(1) {
                if *scale == "ms" {
                    Some(value * 0.001)
                } else if *scale == "s" {
                    Some(value)
                } else if *scale == "m" {
                    Some(value * 60.0)
                } else {
                    None
                }
            } else {
                Some(value)
            }
        })
        .update_while_editing(false)))
}
pub fn time_drag_value(ui: &mut egui::Ui, value: &mut f64) -> egui::Response {
    time_drag_value_builder(ui, value, |a| a)
}
