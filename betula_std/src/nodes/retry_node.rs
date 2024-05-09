use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RetryNodeConfig {
    /// The duration to retry for before returning failure.
    pub time_limit: f64,
}
impl IsNodeConfig for RetryNodeConfig {}

/// Node to retry execution of its child node.
///
/// Returns [`ExecutionStatus::Running`] if the child node returns [`ExecutionStatus::Failure`] for
/// up to the `time_limit` duration. If the child node returns [`ExecutionStatus::Success`], the
/// retry also returns success and the interval is reset. Once the time limit is reached, the
/// interval is reset and a final [`ExecutionStatus::Failure`] is returned.
///
/// One input port `time`, of type `f64`, which usually is time in seconds.
#[derive(Debug, Default)]
pub struct RetryNode {
    time: Input<f64>,
    start_time: Option<f64>,
    pub config: RetryNodeConfig,
}

impl RetryNode {
    pub fn new(time_limit: f64) -> Self {
        RetryNode {
            config: RetryNodeConfig { time_limit },
            ..Default::default()
        }
    }
}

impl Node for RetryNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        if ctx.children() != 1 {
            return Err("RetryNode must have exactly one child node".into());
        }
        // Check if we have actually failed.
        let time = self.time.get()?;
        if let Some(last_start_time) = self.start_time.as_ref() {
            if last_start_time + self.config.time_limit < time {
                self.start_time = None;
                return Ok(ExecutionStatus::Failure);
            }
        } else {
            self.start_time = Some(time);
        }

        Ok(match ctx.run(0)? {
            ExecutionStatus::Success => {
                self.start_time = None;
                ExecutionStatus::Success
            }
            ExecutionStatus::Failure => ExecutionStatus::Running,
            ExecutionStatus::Running => ExecutionStatus::Running,
        })
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![Port::input::<f64>("time")])
    }

    fn setup_inputs(
        &mut self,
        interface: &mut dyn BlackboardInputInterface,
    ) -> Result<(), NodeError> {
        self.time = interface.input::<f64>("time")?;
        Ok(())
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        self.config.load_node_config(config)
    }

    fn reset(&mut self) {
        self.start_time = None;
    }

    fn static_type() -> NodeType {
        "std_retry".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}

#[cfg(feature = "betula_editor")]
pub mod ui_support {
    use super::*;
    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for RetryNode {
        fn ui_title(&self) -> String {
            "retry".to_owned()
        }
        fn ui_icon(&self, ui: &mut egui::Ui, desired_size: egui::Vec2) {
            let _ = desired_size;
            ui.add(egui::Label::new("🔁").selectable(false));
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
                ui.label("TimeLimit: ");
                if betula_editor::egui_util::time_drag_value(ui, &mut self.config.time_limit)
                    .changed()
                {
                    ui_response = UiConfigResponse::Changed;
                }
            });

            ui_response
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            1..1
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("conditional".to_owned()),
                // UiNodeCategory::Group("time".to_owned()),
                UiNodeCategory::Name("retry".to_owned()),
            ]
        }
    }
}
