use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CursorPositionNodeConfig {
    pub windows_offset: CursorPosition,
    pub linux_offset: CursorPosition,
}
impl IsNodeConfig for CursorPositionNodeConfig {}

use crate::{CursorPosition, CursorPositionRetriever};

#[derive(Debug, Default)]
pub struct CursorPositionNode {
    pub config: CursorPositionNodeConfig,
    retriever: CursorPositionRetriever,
    output: Output<CursorPosition>,
}

impl CursorPositionNode {
    pub fn new() -> Self {
        CursorPositionNode::default()
    }
}

impl Node for CursorPositionNode {
    fn execute(&mut self, _ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        let pos = self.retriever.cursor_position()?;
        const IS_WINDOWS: bool = cfg!(target_os = "windows");
        let pos = if IS_WINDOWS {
            CursorPosition {
                x: pos.x + self.config.windows_offset.x,
                y: pos.y + self.config.windows_offset.y,
            }
        } else {
            CursorPosition {
                x: pos.x + self.config.linux_offset.x,
                y: pos.y + self.config.linux_offset.y,
            }
        };

        self.output.set(pos)?;
        Ok(ExecutionStatus::Success)
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![Port::output::<CursorPosition>("cursor")])
    }

    fn setup_outputs(
        &mut self,
        interface: &mut dyn BlackboardOutputInterface,
    ) -> Result<(), NodeError> {
        self.output = interface.output::<CursorPosition>("cursor", CursorPosition::default())?;
        Ok(())
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
            _scale: f32,
        ) -> UiConfigResponse {
            let _ = ctx;
            // let mut ui_response = UiConfigResponse::UnChanged;
            let mut modified = false;
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("linux Δ: ");
                    modified |= ui
                        .add(egui::DragValue::new(&mut self.config.linux_offset.x))
                        .changed();
                    modified |= ui
                        .add(egui::DragValue::new(&mut self.config.linux_offset.y))
                        .changed();
                });
                ui.horizontal(|ui| {
                    ui.label("windows Δ: ");
                    modified |= ui
                        .add(egui::DragValue::new(&mut self.config.windows_offset.x))
                        .changed();
                    modified |= ui
                        .add(egui::DragValue::new(&mut self.config.windows_offset.y))
                        .changed();
                });
            });

            if modified {
                UiConfigResponse::Changed
            } else {
                UiConfigResponse::UnChanged
            }
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
