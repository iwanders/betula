use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct IfTimeExceedsNodeConfig {
    /// The value by which the times must exceed each other.
    pub value: f64,
}
impl IsNodeConfig for IfTimeExceedsNodeConfig {}

/// Node that checks if t1 exceeds t2 by a value.
///
/// Returns [`ExecutionStatus::Failure`] while `(t1 < (t2 + value))` is true, returns
/// [`ExecutionStatus::Success`] or the child value when the comparison is false.
#[derive(Debug, Default)]
pub struct IfTimeExceedsNode {
    t1: Input<f64>,
    t2: Input<f64>,
    pub config: IfTimeExceedsNodeConfig,
}

impl Node for IfTimeExceedsNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        let t1 = self.t1.get()?;
        let t2 = self.t2.get()?;
        if t1 < (t2 + self.config.value) {
            return Ok(ExecutionStatus::Failure);
        } else {
            ctx.decorate_or(ExecutionStatus::Success)
        }
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![Port::input::<f64>("t1"), Port::input::<f64>("t2")])
    }

    fn setup_inputs(
        &mut self,
        interface: &mut dyn BlackboardInputInterface,
    ) -> Result<(), NodeError> {
        self.t1 = interface.input::<f64>("t1")?;
        self.t2 = interface.input::<f64>("t2")?;
        Ok(())
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        self.config.load_node_config(config)
    }

    fn static_type() -> NodeType {
        "std_if_time_exceeds".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}

#[cfg(feature = "betula_editor")]
pub mod ui_support {
    use super::*;
    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for IfTimeExceedsNode {
        fn ui_title(&self) -> String {
            "time exceeds".to_owned()
        }
        fn ui_icon(&self, ui: &mut egui::Ui, desired_size: egui::Vec2) {
            let _ = desired_size;
            ui.add(egui::Label::new("⏰").selectable(false));
        }

        fn ui_config(
            &mut self,
            ctx: &dyn UiNodeContext,
            ui: &mut egui::Ui,
            _scale: f32,
        ) -> UiConfigResponse {
            let _ = ctx;
            let mut ui_response = UiConfigResponse::UnChanged;
            ui.horizontal(|ui| {
                ui.label("t1 <= t2 + : ");
                let r = betula_editor::egui_util::time_drag_value(ui, &mut self.config.value);
                if r.changed() {
                    ui_response = UiConfigResponse::Changed;
                }
            });

            ui_response
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..1
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("conditional".to_owned()),
                // UiNodeCategory::Group("time".to_owned()),
                UiNodeCategory::Name("if_time_exceeds".to_owned()),
            ]
        }
    }
}
