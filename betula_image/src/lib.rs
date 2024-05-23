pub type CaptureError = Box<dyn std::error::Error + Send + Sync + 'static>;
pub type PatternError = CaptureError;

pub mod nodes;
pub mod pattern_match;

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

impl std::ops::Deref for Image {
    type Target = image::RgbaImage;
    fn deref(&self) -> &Self::Target {
        &*self.image
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
        #[derive(Deserialize)]
        pub struct DummyImage {
            width: u32,
            height: u32,
        }
        let t = DummyImage::deserialize(deserializer)?;
        Ok(Image {
            width: t.width,
            height: t.height,
            image: image::RgbaImage::new(0, 0).into(),
        })
    }
}

impl std::fmt::Debug for Image {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "Image({}x{})", self.width, self.height)
    }
}
impl PartialEq for Image {
    fn eq(&self, other: &Image) -> bool {
        self.image.as_ptr() == other.image.as_ptr()
    }
}

#[cfg(feature = "betula_enigo")]
mod enigo_support {
    use super::*;
    use betula_enigo::CursorPosition;

    #[derive(Clone, Serialize, PartialEq, Deserialize, Default)]
    pub struct ImageCursor {
        pub image: Image,
        pub cursor: CursorPosition,
        pub time: f64,
        pub counter: usize,
    }
    impl std::fmt::Debug for ImageCursor {
        fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
            write!(
                fmt,
                "{:?}@({},{})",
                self.image, self.cursor.x, self.cursor.y
            )
        }
    }
}
#[cfg(feature = "betula_enigo")]
pub use enigo_support::ImageCursor;

/// Register nodes to the ui support.
#[cfg(feature = "betula_editor")]
pub fn add_ui_support(ui_support: &mut betula_editor::UiSupport) {
    ui_support.add_value_default_named::<Image>("Image");
    ui_support
        .add_node_default_with_config::<nodes::ImageCaptureNode, nodes::ImageCaptureNodeConfig>();
    ui_support.add_node_default_with_config::<nodes::ImageMatchNode, nodes::ImageMatchNodeConfig>();

    #[cfg(feature = "betula_enigo")]
    {
        use betula_common::callback::CallbacksBlackboard;
        ui_support.add_value_default_named::<ImageCursor>("ImageCursor");
        ui_support.add_value_default_named::<CallbacksBlackboard<ImageCursor>>("ImageCursorCB");
        ui_support
            .add_node_default_with_config::<nodes::ImageCaptureCursorNode, nodes::ImageCaptureNodeConfig>();
        ui_support
            .add_node_default_with_config::<nodes::ImageWriteCursorNode, nodes::ImageWriteCursorNodeConfig>();
    }
}
