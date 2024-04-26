use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

use crate::capture::{get_config, CaptureGrabber, CaptureSpecification};
use crate::CaptureImage;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CaptureNodeConfig {
    force_copy: bool,
    specifications: Vec<CaptureSpecification>,
}
impl IsNodeConfig for CaptureNodeConfig {}

#[derive(Default)]
pub struct CaptureNode {
    output: Output<CaptureImage>,
    capture: Option<CaptureGrabber>,
    pub config: CaptureNodeConfig,
}
impl std::fmt::Debug for CaptureNode {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "CaptureNode")
    }
}

impl CaptureNode {
    pub fn new() -> Self {
        CaptureNode::default()
    }
}

impl Node for CaptureNode {
    fn execute(&mut self, _ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        Ok(ExecutionStatus::Success)
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![Port::output::<CaptureImage>("image")])
    }
    fn setup_outputs(
        &mut self,
        interface: &mut dyn BlackboardOutputInterface,
    ) -> Result<(), NodeError> {
        self.output = interface.output::<CaptureImage>("image", Default::default())?;
        Ok(())
    }

    fn static_type() -> NodeType {
        "capture_node".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        self.config.load_node_config(config)
    }
}

#[cfg(feature = "betula_editor")]
mod ui_support {
    use super::*;
    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for CaptureNode {
        fn ui_title(&self) -> String {
            "capture 📷 ".to_owned()
        }

        fn ui_config(
            &mut self,
            ctx: &dyn UiNodeContext,
            ui: &mut egui::Ui,
            scale: f32,
        ) -> UiConfigResponse {
            let _ = ctx;
            let mut ui_response = UiConfigResponse::UnChanged;
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new("➕")).clicked() {
                        self.config
                            .specifications
                            .push(CaptureSpecification::default());
                        ui_response = UiConfigResponse::Changed;
                    }
                    if ui.add(egui::Button::new("➖")).clicked() {
                        if !self.config.specifications.is_empty() {
                            self.config
                                .specifications
                                .truncate(self.config.specifications.len() - 1);
                            ui_response = UiConfigResponse::Changed;
                        }
                    }
                    let r = ui.checkbox(&mut self.config.force_copy, "Copy");
                    if r.changed() {
                        ui_response = UiConfigResponse::Changed;
                    }
                });

                ui.vertical(|ui| {
                    for (i, t) in self.config.specifications.iter_mut().enumerate() {
                        ui.horizontal(|ui| {});
                    }
                });
            });

            ui_response
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("action".to_owned()),
                UiNodeCategory::Name("capture".to_owned()),
            ]
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..0
        }
    }
}
