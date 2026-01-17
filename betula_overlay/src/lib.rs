pub type OverlayError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub mod nodes;

use serde::{Deserialize, Serialize};

use screen_overlay::{Overlay, OverlayConfig, OverlayHandle};

use std::sync::Arc;
#[derive(Clone)]
pub struct OverlayInterface {
    pub overlay: OverlayHandle,
}
impl std::fmt::Debug for OverlayInterface {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "OverlayInterface")
    }
}

impl std::ops::Deref for OverlayInterface {
    type Target = OverlayHandle;
    fn deref(&self) -> &Self::Target {
        &self.overlay
    }
}

impl std::cmp::PartialEq for OverlayInterface {
    fn eq(&self, other: &OverlayInterface) -> bool {
        self == other
    }
}

impl OverlayInterface {
    pub fn new() -> Result<Self, OverlayError> {
        let (width, height) = (1920.0, 1080.0);
        let (x, y) = (0.0, 0.0);
        let config = OverlayConfig::new()
            .with_size([width, height])
            .with_position([x, y])
            .with_central_panel_fill(screen_overlay::egui::Color32::TRANSPARENT);
        let overlay = Overlay::new(config);
        let overlay = OverlayHandle::new(overlay);

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
    ui_support.add_node_default_with_config::<nodes::OverlayInstanceNode, nodes::OverlayInstanceNodeConfig>();
    ui_support
        .add_node_default_with_config::<nodes::OverlayTextNode, nodes::OverlayTextNodeConfig>();
}
