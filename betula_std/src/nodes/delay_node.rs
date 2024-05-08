use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DelayNodeConfig {
    /// The interval to wait between executions in 'time' unit.
    pub interval: f64,
}
impl IsNodeConfig for DelayNodeConfig {}

/// Node to delay execution of its children.
///
/// Returns [`ExecutionStatus::Running`] while the interval since the last
/// execution is not yet exceeded. When the interval is exceeded it runs
/// its one child node and returns its status. If there is no child node
/// to be executed it returns [`ExecutionStatus::Running`].
///
/// One input port `time`, of type `f64`, which usually is time in seconds.
#[derive(Debug, Default)]
pub struct DelayNode {
    time: Input<f64>,
    last_time: f64,
    pub config: DelayNodeConfig,
}

impl DelayNode {
    pub fn new(interval: f64) -> Self {
        DelayNode {
            config: DelayNodeConfig { interval },
            ..Default::default()
        }
    }
}

impl Node for DelayNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        let time = self.time.get()?;
        if time < (self.last_time + self.config.interval) {
            return Ok(ExecutionStatus::Running);
        }
        self.last_time = time;

        if ctx.children() == 1 {
            ctx.run(0)
        } else {
            Ok(ExecutionStatus::Success)
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

    fn reset(&mut self) {
        self.last_time = 0.0;
    }

    fn static_type() -> NodeType {
        "common_delay".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}

#[cfg(feature = "betula_editor")]
pub mod ui_support {
    use super::*;
    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for DelayNode {
        fn ui_title(&self) -> String {
            "delay".to_owned()
        }
        fn ui_icon(&self, ui: &mut egui::Ui) {
            ui.add(egui::Label::new("⏱").selectable(false));
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
                ui.label("Delay: ");

                let speed = if self.config.interval < 1.0 {
                    0.001
                } else if self.config.interval < 10.0 {
                    0.01
                } else {
                    0.1
                };

                let r = ui.add(
                    egui::DragValue::new(&mut self.config.interval)
                        .clamp_range(0.0f64..=(24.0 * 60.0 * 60.0))
                        .speed(speed)
                        .custom_formatter(|v, _| {
                            if v < 10.0 {
                                format!("{:.0} ms", v * 1000.0)
                            } else if v < 60.0 {
                                format!("{:.3} s", v)
                            } else {
                                format!("{:.0} s", v)
                            }
                        })
                        .custom_parser(|s| {
                            let parts: Vec<&str> = s.split(' ').collect();
                            let value = parts[0].parse::<f64>().ok()?;
                            if let Some(scale) = parts.get(1) {
                                if *scale == "ms" {
                                    Some(value * 0.001)
                                } else if *scale == "s" {
                                    Some(value)
                                } else if *scale == "m" {
                                    Some(value * 60.0)
                                } else {
                                    None
                                }
                            } else {
                                Some(value)
                            }
                        })
                        .update_while_editing(false),
                );

                if r.changed() {
                    // println!("Changed! now: {}", self.config.interval);
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
                UiNodeCategory::Name("delay".to_owned()),
            ]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_blackboard_reqs() -> Result<(), NodeError> {
        use betula_core::{
            basic::{BasicBlackboard, BasicTree},
            NodeId,
        };
        use uuid::Uuid;
        let mut tree = BasicTree::new();
        let mut bb = BasicBlackboard::default();
        let time = bb.output::<f64>("time", 0.0)?;
        time.set(1.0)?;

        let root = tree.add_node_boxed(NodeId(Uuid::new_v4()), Box::new(DelayNode::new(5.0)))?;
        let ports = tree.node_mut(root).ok_or("node not found")?.ports()?;
        assert!(ports.len() == 1);
        tree.node_mut(root)
            .ok_or("node not found")?
            .setup_inputs(&mut bb)?;
        assert!(tree.execute(root)? == ExecutionStatus::Running);
        time.set(2.0)?;
        assert!(tree.execute(root)? == ExecutionStatus::Running);
        time.set(6.0)?;
        assert!(tree.execute(root)? == ExecutionStatus::Success);
        assert!(tree.execute(root)? == ExecutionStatus::Running);
        time.set(7.0)?;
        assert!(tree.execute(root)? == ExecutionStatus::Running);
        time.set(10.0)?;
        assert!(tree.execute(root)? == ExecutionStatus::Running);
        time.set(11.0)?;
        assert!(tree.execute(root)? == ExecutionStatus::Success);
        assert!(tree.execute(root)? == ExecutionStatus::Running);
        Ok(())
    }
}
