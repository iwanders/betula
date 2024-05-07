use betula_core::node_prelude::*;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct SequenceNodeConfig {
    /// Whether to keep memory between cycles and continue from the previous state.
    ///
    /// Default is off, which makes it a reactive node; each cycle all nodes are executed from the
    /// start.
    pub memory: bool,
}
impl IsNodeConfig for SequenceNodeConfig {}

/// Node that executes nodes in sequence until one does not return [`ExecutionStatus::Success`].
///
/// Runs nodes from left to right, ignoring [`ExecutionStatus::Success`] but
/// returning the first [`ExecutionStatus::Failure`] or [`ExecutionStatus::Running`]
/// encountered, at this point that value is returned.
/// The node returns [`ExecutionStatus::Success`] if all child nodes succceed.
#[derive(Debug, Copy, Clone, Default)]
pub struct SequenceNode {
    pub config: SequenceNodeConfig,
    current_position: usize,
}
impl Node for SequenceNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        if self.current_position >= ctx.children() {
            self.current_position = 0;
        }
        let previous = if self.config.memory {
            self.current_position
        } else {
            0
        };
        // println!("Previous: {previous}");

        for id in 0..ctx.children() {
            if id < previous {
                // println!("Skipping as {id} < {previous}");
                continue; // done in a prior cycle.
            }
            match ctx.run(id)? {
                ExecutionStatus::Success => {
                    // Advance the sequence up to this point.
                    // println!("current_position: {id}");
                    self.current_position = id + 1;
                }
                ExecutionStatus::Failure => {
                    // Reset the sequence.
                    self.current_position = 0;
                    // println!("current_position: 0");
                    return Ok(ExecutionStatus::Failure);
                }
                ExecutionStatus::Running => {
                    // No action, next cycle we would run this again.
                    return Ok(ExecutionStatus::Running);
                }
            }
        }

        // All children succeeded.
        Ok(ExecutionStatus::Success)
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        self.config.load_node_config(config)
    }

    fn reset(&mut self) {
        self.current_position = 0;
    }

    fn static_type() -> NodeType
    where
        Self: Sized,
    {
        "sequence".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}

#[cfg(feature = "betula_editor")]
pub mod ui_support {
    use super::*;
    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for SequenceNode {
        fn ui_title(&self) -> String {
            "sequence ⮊".to_owned()
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
                let r = r.on_hover_text("Check this to continue execution where the previous cycle returned, if false the node is reactive and resets each cycle");
                modified |= r.changed();
            });

            if modified {
                UiConfigResponse::Changed
            } else {
                UiConfigResponse::UnChanged
            }
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("control".to_owned()),
                // UiNodeCategory::Group("core".to_owned()),
                UiNodeCategory::Name("sequence".to_owned()),
            ]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::{FailureNode, SequenceNode};
    use betula_core::{basic::BasicTree, NodeId};
    use uuid::Uuid;

    #[test]
    fn sequence_fail() -> Result<(), NodeError> {
        let mut tree = BasicTree::new();
        let root = tree.add_node_boxed(NodeId(Uuid::new_v4()), Box::new(SequenceNode {}))?;
        let f1 = tree.add_node_boxed(NodeId(Uuid::new_v4()), Box::new(FailureNode {}))?;
        tree.set_children(root, &vec![f1])?;
        let res = tree.execute(root)?;
        assert_eq!(res, ExecutionStatus::Failure);
        Ok(())
    }
}
