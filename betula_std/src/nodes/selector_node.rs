use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct SelectorNodeConfig {
    /// Whether to keep memory between cycles and continue from the previous state.
    ///
    /// Default is off, which makes it a reactive node; each cycle all nodes are executed from the
    /// start. Memory false is reactive, memory true is normal selector/fallback.
    pub memory: bool,
}
impl IsNodeConfig for SelectorNodeConfig {}

/// Node that executes nodes in sequence returning the first non-[`ExecutionStatus::Failure`].
///
/// Runs nodes from left to right, ignoring [`ExecutionStatus::Failure`] but
/// returning the first [`ExecutionStatus::Success`] or [`ExecutionStatus::Running`]
/// encountered, at this point that value is returned.
/// The node returns [`ExecutionStatus::Failure`] if all child nodes failed.
#[derive(Debug, Copy, Clone, Default)]
pub struct SelectorNode {
    pub config: SelectorNodeConfig,
    current_position: usize,
}
impl Node for SelectorNode {
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
                ExecutionStatus::Failure => {
                    // Advance the sequence up to this point.
                    // println!("current_position: {id}");
                    self.current_position = id + 1;
                    ctx.reset_recursive(id)?;
                }
                ExecutionStatus::Success => {
                    // Reset the sequence, resetting all previous children.
                    self.current_position = 0;
                    for i in 0..=id {
                        ctx.reset_recursive(i)?;
                    }
                    // println!("current_position: 0");
                    return Ok(ExecutionStatus::Success);
                }
                ExecutionStatus::Running => {
                    if !self.config.memory {
                        // Precursors already got reset in failure call, so only need to reset the
                        // current one here.
                        ctx.reset_recursive(id)?;
                    }
                    // No action, next cycle we would run this again.
                    return Ok(ExecutionStatus::Running);
                }
            }
        }

        // Reached here, all children must've failed, reset them and return failure.
        ctx.reset_children()?;

        Ok(ExecutionStatus::Failure)
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
        "selector".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}

#[cfg(feature = "betula_editor")]
pub mod ui_support {
    use super::*;
    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for SelectorNode {
        fn ui_title(&self) -> String {
            "selector ⛶".to_owned()
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
                UiNodeCategory::Name("selector".to_owned()),
            ]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::{FailureNode, SuccessNode};
    use betula_core::{basic::BasicTree, NodeId};
    use uuid::Uuid;

    #[test]
    fn selector_success() -> Result<(), NodeError> {
        let mut tree = BasicTree::new();
        let root =
            tree.add_node_boxed(NodeId(Uuid::new_v4()), Box::new(SelectorNode::default()))?;
        let f1 = tree.add_node_boxed(NodeId(Uuid::new_v4()), Box::new(FailureNode {}))?;
        let s1 = tree.add_node_boxed(NodeId(Uuid::new_v4()), Box::new(SuccessNode {}))?;
        tree.set_children(root, &vec![f1, s1])?;
        let res = tree.execute(root)?;
        assert_eq!(res, ExecutionStatus::Success);
        Ok(())
    }
}
