use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

use super::ImageCaptureNode;

#[derive(Default)]
pub struct ImageCaptureCursorNode {
    node: ImageCaptureNode,
}
impl std::fmt::Debug for ImageCaptureCursorNode {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "ImageCaptureCursorNode")
    }
}

impl Node for ImageCaptureCursorNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        self.node.execute(ctx)
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        self.node.ports()
    }
    fn setup_outputs(
        &mut self,
        interface: &mut dyn BlackboardOutputInterface,
    ) -> Result<(), NodeError> {
        self.node.setup_outputs(interface)
    }

    fn static_type() -> NodeType {
        "image_capture_cursor".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        self.node.get_config()
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        self.node.set_config(config)
    }

    fn reset(&mut self) {
        self.node.reset();
    }
}

#[cfg(feature = "betula_editor")]
mod ui_support {
    use super::*;
    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for ImageCaptureCursorNode {
        fn ui_title(&self) -> String {
            "capture cursor 📷 ".to_owned()
        }

        fn ui_config(
            &mut self,
            ctx: &dyn UiNodeContext,
            ui: &mut egui::Ui,
            scale: f32,
        ) -> UiConfigResponse {
            self.node.ui_config(ctx, ui, scale)
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("provider".to_owned()),
                UiNodeCategory::Name("image_capture_cursor".to_owned()),
            ]
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..0
        }
    }
}
