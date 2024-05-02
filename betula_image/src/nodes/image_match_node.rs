use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

use crate::Image;

use crate::pattern_match::PatternName;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ImageMatchNodeConfig {
    use_match: Option<PatternName>,
}

impl IsNodeConfig for ImageMatchNodeConfig {}

#[derive(Default)]
pub struct ImageMatchNode {
    /// Actual input image.
    input: Input<Image>,

    /// The image pattern against which is to be matched.
    config: ImageMatchNodeConfig,

    /// The directory from which the image is loaded.
    directory: Option<std::path::PathBuf>,

    /// The actual pattern against which is being matched.
    pattern: Option<crate::pattern_match::Pattern>,
}
impl std::fmt::Debug for ImageMatchNode {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "ImageMatchNode")
    }
}

impl ImageMatchNode {
    pub fn new() -> Self {
        ImageMatchNode::default()
    }
}

impl Node for ImageMatchNode {
    fn execute(&mut self, _ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        todo!();
        Ok(ExecutionStatus::Success)
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![Port::input::<Image>("image")])
    }
    fn setup_inputs(
        &mut self,
        interface: &mut dyn BlackboardInputInterface,
    ) -> Result<(), NodeError> {
        self.input = interface.input::<Image>("image")?;
        Ok(())
    }

    fn static_type() -> NodeType {
        "image_match".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        let r = self.config.load_node_config(config);
        r
    }

    fn set_directory(&mut self, directory: Option<&std::path::Path>) {
        self.directory = directory.map(|v| v.to_owned());
    }

    fn reset(&mut self) {}
}

#[cfg(feature = "betula_editor")]
mod ui_support {
    use super::*;
    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for ImageMatchNode {
        fn ui_title(&self) -> String {
            "image_match 📷 ".to_owned()
        }

        fn ui_config(
            &mut self,
            ctx: &dyn UiNodeContext,
            ui: &mut egui::Ui,
            scale: f32,
        ) -> UiConfigResponse {
            let _ = (ctx, scale);

            let mut modified = false;

            if modified {
                UiConfigResponse::Changed
            } else {
                UiConfigResponse::UnChanged
            }
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("action".to_owned()),
                UiNodeCategory::Name("image_match".to_owned()),
            ]
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..0
        }
    }
}
