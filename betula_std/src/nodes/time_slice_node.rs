use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeSliceNodeConfig {
    /// Period of the entire time window.
    pub period: f64,
    /// The duration of the time slice this node gets in the window.
    pub duration: f64,
    /// Offset of the slice in the period.
    pub offset: f64,
}
impl IsNodeConfig for TimeSliceNodeConfig {}

impl Default for TimeSliceNodeConfig {
    fn default() -> Self {
        Self {
            period: 1.0,
            duration: 0.5,
            offset: 0.0,
        }
    }
}

/// A node that only runs within the specified time slice.
///
/// This can be used to provide a simple way to ensure that two things don't run at the same time.
/// The execution can only happen when the time is within the current window. The time is
/// modulo'd by the period that ensures that all nodes agree on when the interval starts. Individual
/// time slice nodes then only allow execution withen time is within
/// `window_offset <= time <=  (window_offset + window_duration)`.
///
/// The node returns running whenever outside of the time window, inside the time window it calls
/// the one child node and returns its state.
///
/// One input port `time`, of type `f64`, which usually is time in seconds.
#[derive(Debug, Default)]
pub struct TimeSliceNode {
    time: Input<f64>,
    pub config: TimeSliceNodeConfig,
}

impl Node for TimeSliceNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        if ctx.children() != 1 {
            return Err("TimeSliceNode must have exactly one child node".into());
        }

        let time = self.time.get()?;

        let time_in_period = time.rem_euclid(self.config.period);

        let beyond_start = self.config.offset <= time_in_period;
        let before_end = time_in_period <= (self.config.offset + self.config.duration);

        if beyond_start && before_end {
            ctx.run(0)
        } else {
            Ok(ExecutionStatus::Running)
        }
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

    fn static_type() -> NodeType {
        "std_time_slice".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}

#[cfg(feature = "betula_editor")]
pub mod ui_support {
    use super::*;
    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for TimeSliceNode {
        fn ui_title(&self) -> String {
            "time slice".to_owned()
        }
        fn ui_icon(&self, ui: &mut egui::Ui, desired_size: egui::Vec2) {
            let _ = desired_size;
            ui.add(egui::Label::new("🍕").selectable(false));
        }

        fn ui_config(&mut self, ctx: &dyn UiNodeContext, ui: &mut egui::Ui) -> UiConfigResponse {
            let _ = ctx;
            let mut modified = false;

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Period: ");
                    let r = betula_editor::egui_util::time_drag_value(ui, &mut self.config.period);
                    modified |= r
                        .on_hover_text("The window period in which the time slices are placed.")
                        .changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Window: ");
                    let r = betula_editor::egui_util::time_drag_value_builder(
                        ui,
                        &mut self.config.duration,
                        |a| a.clamp_range(0.0f64..=(self.config.period - self.config.offset)),
                    );
                    modified |= r
                        .on_hover_text("The duration of the time slice in the window.")
                        .changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Offset: ");
                    let r = betula_editor::egui_util::time_drag_value_builder(
                        ui,
                        &mut self.config.offset,
                        |a| a.clamp_range(0.0f64..=(self.config.period - self.config.duration)),
                    );
                    modified |= r
                        .on_hover_text("The offset of the window in the period.")
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
            0..1
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("conditional".to_owned()),
                // UiNodeCategory::Group("time".to_owned()),
                UiNodeCategory::Name("time_slice".to_owned()),
            ]
        }
    }
}
