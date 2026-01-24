pub type OverlayError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub mod nodes;

use serde::{Deserialize, Serialize};

use screen_overlay::{Overlay, OverlayConfig, OverlayHandle, VisualId};

#[cfg(feature = "use_client_server")]
pub mod client_server;

#[derive(Clone, Debug)]
enum OverlayImpl {
    Local(OverlayHandle),
    #[cfg(feature = "use_client_server")]
    Remote(client_server::OverlayClient),
}
pub(crate) trait OverlaySupport {
    fn set_text(
        &self,
        text_config: &nodes::OverlayTextNodeConfig,
        text: &str,
    ) -> Result<VisualId, OverlayError>;
    fn remove_id(&self, id: &VisualId) -> Result<(), OverlayError>;
    fn remove_and_set_text(
        &self,
        id: &VisualId,
        text_config: &nodes::OverlayTextNodeConfig,
        text: &str,
    ) -> Result<VisualId, OverlayError>;
}
impl OverlaySupport for OverlayImpl {
    fn set_text(
        &self,
        text_config: &nodes::OverlayTextNodeConfig,
        text: &str,
    ) -> Result<VisualId, OverlayError> {
        match self {
            OverlayImpl::Local(overlay_handle) => {
                use screen_overlay::PositionedElements;

                let font_size = text_config.font_size;
                let text_color = text_config.text_color;
                let text_clone = text.to_owned();
                let drawable = PositionedElements::new()
                    .fixed_pos(egui::pos2(
                        text_config.position.0 as f32,
                        text_config.position.1 as f32,
                    ))
                    .default_size(egui::vec2(
                        text_config.size.0 as f32,
                        text_config.size.1 as f32,
                    ))
                    .fill(text_config.fill_color)
                    .add_closure(move |ui| {
                        let text = egui::widget_text::RichText::new(&text_clone)
                            .size(font_size)
                            .color(text_color);
                        ui.label(text);
                    });

                let text_token = overlay_handle.add_drawable(drawable.into());
                Ok(text_token.into_id())
            }
            #[cfg(feature = "use_client_server")]
            OverlayImpl::Remote(overlay_client) => {
                overlay_client.add_text(text_config.to_instruction(text))
            }
        }
    }
    fn remove_id(&self, id: &VisualId) -> Result<(), OverlayError> {
        match self {
            OverlayImpl::Local(overlay_handle) => {
                overlay_handle.remove_element(*id);
                Ok(())
            }
            #[cfg(feature = "use_client_server")]
            OverlayImpl::Remote(overlay_client) => overlay_client.remove(*id),
        }
    }

    fn remove_and_set_text(
        &self,
        id: &VisualId,
        text_config: &nodes::OverlayTextNodeConfig,
        text: &str,
    ) -> Result<VisualId, OverlayError> {
        match self {
            OverlayImpl::Local(_) => {
                let _ = self.remove_id(id)?;
                self.set_text(text_config, text)
            }
            #[cfg(feature = "use_client_server")]
            OverlayImpl::Remote(overlay_client) => {
                let results = overlay_client.request(&[
                    client_server::OverlayRequest {
                        command: client_server::RequestCommand::Remove(*id),
                    },
                    client_server::OverlayRequest {
                        command: client_server::RequestCommand::Add(client_server::Drawable::Text(
                            text_config.to_instruction(text),
                        )),
                    },
                ])?;
                // Last one has the id.
                results.last().unwrap().command.response_add()
            }
        }
    }
}

#[derive(Clone)]
pub struct OverlayInterface {
    overlay: OverlayImpl,
}
impl std::fmt::Debug for OverlayInterface {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "OverlayInterface")
    }
}

impl std::cmp::PartialEq for OverlayInterface {
    fn eq(&self, other: &OverlayInterface) -> bool {
        self == other
    }
}

impl OverlayInterface {
    pub fn new_local(config: OverlayConfig) -> Result<Self, OverlayError> {
        // let (width, height) = (1920.0, 1080.0);
        // let (x, y) = (0.0, 0.0);
        // let config = OverlayConfig::new()
        //     .with_size([width, height])
        //     .with_position([x, y])
        //     .with_central_panel_fill(screen_overlay::egui::Color32::TRANSPARENT);

        let overlay = Overlay::new(config);
        let overlay = OverlayHandle::new(overlay);

        register_overlay(&overlay);

        Ok(OverlayInterface {
            overlay: OverlayImpl::Local(overlay),
        })
    }
    #[cfg(feature = "use_client_server")]
    pub fn new_remote(clear: bool, config: OverlayConfig) -> Result<Self, OverlayError> {
        let overlay = client_server::OverlayClient::new(Default::default());
        overlay.set_config(&config)?;
        if clear {
            overlay.remove_all_elements()?;
        }
        Ok(OverlayInterface {
            overlay: OverlayImpl::Remote(overlay),
        })
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

// We need this machinery here to track overlays that exist, such that in the ui service loop we can service the overlays.

type WeakOverlay = std::sync::Weak<Overlay>;
static OVERLAYS_IN_EXISTANCE: std::sync::LazyLock<std::sync::Mutex<Vec<WeakOverlay>>> =
    std::sync::LazyLock::<std::sync::Mutex<Vec<WeakOverlay>>>::new(|| Default::default());

fn register_overlay(overlay: &OverlayHandle) {
    let mut overlays = OVERLAYS_IN_EXISTANCE.lock().unwrap();
    overlays.push(overlay.to_weak())
}

pub fn get_overlays() -> Vec<OverlayHandle> {
    let mut overlays = OVERLAYS_IN_EXISTANCE.lock().unwrap();
    let strong: Vec<OverlayHandle> = overlays
        .drain(..)
        .filter_map(screen_overlay::OverlayHandle::from_weak)
        .collect();

    // Re-assign the ones that still exist.
    for s in strong.iter() {
        overlays.push(s.to_weak());
    }

    strong
}

/// Register nodes to the ui support.
#[cfg(feature = "betula_editor")]
pub fn add_ui_support(ui_support: &mut betula_editor::UiSupport) {
    ui_support.add_value_default_named::<OverlayBlackboard>("OverlayBlackboard");
    ui_support.add_node_default_with_config::<nodes::OverlayInstanceNode, nodes::OverlayInstanceNodeConfig>();
    ui_support
        .add_node_default_with_config::<nodes::OverlayTextNode, nodes::OverlayTextNodeConfig>();
}
