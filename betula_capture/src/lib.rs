pub type CaptureError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub mod nodes;

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Serialize)]
pub struct CaptureImage {
    #[serde(skip)]
    pub image: std::sync::Arc<image::RgbaImage>,
}
impl Default for CaptureImage {
    fn default() -> Self {
        // superb hack here... we make an image that's 0x0 pixels.
        let dummy = image::RgbaImage::new(0, 0);
        CaptureImage {
            image: std::sync::Arc::new(dummy),
        }
    }
}

impl<'de> Deserialize<'de> for CaptureImage {
    fn deserialize<D>(deserializer: D) -> Result<CaptureImage, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(CaptureImage::default())
    }
}

impl std::fmt::Debug for CaptureImage {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "Image({}x{})", self.image.width(), self.image.height())
    }
}

impl PartialEq for CaptureImage {
    fn eq(&self, _: &CaptureImage) -> bool {
        false
    }
}

/// Register nodes to the ui support.
#[cfg(feature = "betula_editor")]
pub fn add_ui_support(ui_support: &mut betula_editor::UiSupport) {
    ui_support.add_value_default_named::<CaptureImage>("Image");
    ui_support.add_node_default_with_config::<nodes::CaptureNode, nodes::CaptureNodeConfig>();
}
