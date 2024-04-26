pub type CaptureError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub mod nodes;

pub mod capture;

use serde::{Deserialize, Deserializer, Serialize};

use screen_capture::Image;

#[derive(Clone, Serialize)]
pub struct CaptureImage {
    #[serde(skip)]
    pub image: std::sync::Arc<Box<dyn Image>>,
}
impl Default for CaptureImage {
    fn default() -> Self {
        // superb hack here... we make an image that's 0x0 pixels.
        use screen_capture::raster_image::RasterImage;
        let dummy = RasterImage::filled(0, 0, screen_capture::RGB::black());
        CaptureImage {
            image: std::sync::Arc::new(Box::new(dummy)),
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
        write!(
            fmt,
            "Image({}x{})",
            self.image.get_width(),
            self.image.get_height()
        )
    }
}

impl PartialEq for CaptureImage {
    fn eq(&self, _: &CaptureImage) -> bool {
        false
    }
}

/// Register nodes to the ui support.
#[cfg(feature = "betula_editor")]
pub fn add_ui_support(ui_support: &mut betula_editor::UiSupport) {}
