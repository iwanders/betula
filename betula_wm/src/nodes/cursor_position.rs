use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CursorPositionNodeConfig {
    pub matches: Vec<String>,
}
impl IsNodeConfig for CursorPositionNodeConfig {}

use crate::CursorPositionRetriever;

#[derive(Debug, Default)]
pub struct CursorPositionNode {
    pub config: CursorPositionNodeConfig,
    retriever: CursorPositionRetriever,
}

impl CursorPositionNode {
    pub fn new() -> Self {
        CursorPositionNode::default()
    }
}

impl Node for CursorPositionNode {
    fn execute(&mut self, _ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        let pos = self.retriever.cursor_position()?;
        println!("pos: {pos:?}");
        Ok(ExecutionStatus::Failure)
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        self.config.load_node_config(config)?;
        Ok(())
    }
    fn static_type() -> NodeType {
        "cursor_position".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}

#[cfg(feature = "betula_egui")]
pub mod ui_support {
    use super::*;
    use betula_egui::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for CursorPositionNode {
        fn ui_title(&self) -> String {
            "cursor_position ⬉".to_owned()
        }

        fn ui_config(
            &mut self,
            ctx: &dyn UiNodeContext,
            ui: &mut egui::Ui,
            scale: f32,
        ) -> UiConfigResponse {
            let _ = ctx;
            let mut ui_response = UiConfigResponse::UnChanged;

            ui_response
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..0
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("provider".to_owned()),
                UiNodeCategory::Name("cursor_position".to_owned()),
            ]
        }
    }
}
