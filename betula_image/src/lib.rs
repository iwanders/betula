pub type CaptureError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub mod nodes;

use serde::{Deserialize, Deserializer, Serialize};

use std::sync::Arc;
#[derive(Clone, Serialize)]
pub struct Image {
    width: u32,
    height: u32,
    #[serde(skip)]
    pub image: Arc<image::RgbaImage>,
}

impl Image {
    pub fn new<T: Into<Arc<image::RgbaImage>>>(image: T) -> Self {
        let image: Arc<image::RgbaImage> = image.into();
        Self {
            width: image.width(),
            height: image.height(),
            image,
        }
    }
}

impl Default for Image {
    fn default() -> Self {
        // superb hack here... we make an image that's 0x0 pixels.
        let dummy = image::RgbaImage::new(0, 0);
        Self::new(dummy)
    }
}

impl<'de> Deserialize<'de> for Image {
    fn deserialize<D>(deserializer: D) -> Result<Image, D::Error>
    where
        D: Deserializer<'de>,
    {
        let _ = deserializer;
        Ok(Image::default())
    }
}

impl std::fmt::Debug for Image {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "Image({}x{})", self.width, self.height)
    }
}

impl PartialEq for Image {
    fn eq(&self, other: &Image) -> bool {
        false
    }
}

/// Register nodes to the ui support.
#[cfg(feature = "betula_editor")]
pub fn add_ui_support(ui_support: &mut betula_editor::UiSupport) {
    ui_support.add_value_default_named::<Image>("Image");
    ui_support.add_node_default_with_config::<nodes::CaptureNode, nodes::CaptureNodeConfig>();
}
