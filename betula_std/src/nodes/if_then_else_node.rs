use betula_core::node_prelude::*;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct IfThenElseNodeConfig {
    /// Whether to keep the conditional latched until the child returns non-running.
    ///
    /// Default is off, which makes it a reactive node; each execution the condition is executed
    /// fresh. If on, the condition is latched until the selected child returns
    /// [`ExecutionStatus::Success`] or [`ExecutionStatus::Failure`].
    #[serde(default)]
    pub memory: bool,
}
impl IsNodeConfig for IfThenElseNodeConfig {}

/// Node to do an if(then else) statement.
///
/// The node will execute the first child, if the status is [`ExecutionStatus::Running`] it will
/// return running.
/// If the first child returns [`ExecutionStatus::Success`], the second child is executed and its
/// status returned, if the first child returns  [`ExecutionStatus::Failure`], if the third child
/// exists, it it executed and its status is returned, else it returns [`ExecutionStatus::Failure`].
#[derive(Debug, Default)]
pub struct IfThenElseNode {
    pub config: IfThenElseNodeConfig,
    child: Option<ExecutionStatus>,
}

impl Node for IfThenElseNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        if ctx.children() < 2 || ctx.children() > 3 {
            return Err("IfThenElseNode must have two or three child nodes".into());
        }

        // If we don't have memory, reset the if statement.
        if !self.config.memory {
            self.child = None;
        }

        // Determine the status of the branch
        let status = if let Some(status) = self.child {
            status
        } else {
            let s = ctx.run(0)?;
            if s == ExecutionStatus::Running {
                return Ok(ExecutionStatus::Running);
            }
            self.child = Some(s);
            s
        };

        let r = match status {
            ExecutionStatus::Success => ctx.run(1),
            ExecutionStatus::Failure => {
                if ctx.children() == 3 {
                    ctx.run(2)
                } else {
                    Ok(ExecutionStatus::Failure)
                }
            }
            ExecutionStatus::Running => Ok(ExecutionStatus::Running),
        }?;

        // Reached the end of our if statement.
        if r != ExecutionStatus::Running {
            ctx.reset_children()?;
        }
        Ok(r)
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        self.config.load_node_config(config)
    }

    fn static_type() -> NodeType {
        "std_if_then_else".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }

    fn reset(&mut self) {
        self.child = None;
    }
}

#[cfg(feature = "betula_editor")]
pub mod ui_support {
    use super::*;
    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for IfThenElseNode {
        fn ui_title(&self) -> String {
            "if".to_owned()
        }
        fn ui_icon(&self, ui: &mut egui::Ui, desired_size: egui::Vec2) {
            let _ = desired_size;
            ui.add(egui::Label::new("?").selectable(false));
        }
        fn ui_config(
            &mut self,
            ctx: &dyn UiNodeContext,
            ui: &mut egui::Ui,
            _scale: f32,
        ) -> UiConfigResponse {
            let _ = ctx;
            let mut modified = false;
            ui.horizontal(|ui| {
                let r = ui.checkbox(&mut self.config.memory, "Memory");
                let r = r.on_hover_text(
                    "Check this to keep the branching status until the choice returns non-running.",
                );
                modified |= r.changed();
            });

            if modified {
                UiConfigResponse::Changed
            } else {
                UiConfigResponse::UnChanged
            }
        }

        fn ui_child_range(&self) -> std::ops::Range<usize> {
            2..3
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("control".to_owned()),
                UiNodeCategory::Name("if_then_else".to_owned()),
            ]
        }
    }
}
