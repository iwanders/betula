pub type OverlayError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub mod nodes;

use serde::{Deserialize, Deserializer, Serialize};

use screen_overlay::{Overlay, OverlayConfig};

use std::sync::{Arc, Mutex};
#[derive(Clone)]
pub struct OverlayInterface {
    overlay: Arc<Overlay>,
}
impl std::fmt::Debug for OverlayInterface {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "OverlayInterface")
    }
}

impl std::cmp::PartialEq for OverlayInterface {
    fn eq(&self, other: &OverlayInterface) -> bool {
        Arc::as_ptr(&self.overlay) == Arc::as_ptr(&other.overlay)
    }
}

impl OverlayInterface {
    pub fn new() -> Result<Self, OverlayError> {
        screen_overlay::setup()?;
        let v = Overlay::new_with_config(&OverlayConfig {
            task_bar: false,
            on_top: true,
            name: "Overlay".to_owned(),
            ..Default::default()
        })?;
        let overlay = Arc::new(v);

        Ok(OverlayInterface { overlay })
    }
}

#[derive(Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OverlayBlackboard {
    #[serde(skip)]
    pub interface: Option<OverlayInterface>,
}
impl std::fmt::Debug for OverlayBlackboard {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "Overlay")
    }
}

/// Register nodes to the ui support.
#[cfg(feature = "betula_editor")]
pub fn add_ui_support(ui_support: &mut betula_editor::UiSupport) {
    ui_support.add_value_default_named::<OverlayBlackboard>("OverlayBlackboard");
    // ui_support.add_node_default_with_config::<nodes::ImageMatchNode, nodes::ImageMatchNodeConfig>();
}
