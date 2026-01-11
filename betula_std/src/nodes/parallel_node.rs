use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ParallelNodeConfig {
    /// If this at least this many children return Success, report Success.
    pub success_threshold: usize,

    /// Whether to keep memory between cycles and continue from the previous state.
    ///
    /// Default is off, which starts with no prior knowledge and just executes all children and then
    /// determines the return state. If on, children that previously returned a status other than
    /// running will not be executed again.
    #[serde(default)]
    pub memory: bool,
}

impl IsNodeConfig for ParallelNodeConfig {}

/// Node for parallel execution of its children.
///
/// All children are executed, if the number of children returning success
/// exceeds the `success_threshold`, it returns [`ExecutionStatus::Success`], if
/// the success status can no longer be achieved it returns
/// [`ExecutionStatus::Failure`],
/// in other situations it returns [`ExecutionStatus::Running`].
#[derive(Debug, Default)]
pub struct ParallelNode {
    pub config: ParallelNodeConfig,
    statuses: Vec<ExecutionStatus>,
}

impl Node for ParallelNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        // Should this node have state and only continue executing nodes that have not yet returned
        // success or failure?
        let mut success_count = 0;
        let mut failure_count = 0;
        let n = ctx.children();

        // Ensure statuses is the correct size.
        self.statuses.resize(n, ExecutionStatus::Running);

        // If we don't have memory, just clear the status vector.
        if !self.config.memory {
            self.statuses.fill(ExecutionStatus::Running);
        }

        // Then, tick children and determine count.
        for id in 0..n {
            if self.statuses[id] == ExecutionStatus::Running {
                let node_res = ctx.run(id)?;
                self.statuses[id] = node_res
            }

            match self.statuses[id] {
                ExecutionStatus::Success => success_count += 1,
                ExecutionStatus::Failure => failure_count += 1,
                ExecutionStatus::Running => {}
            }
        }

        // Lets say three nodes.
        // Success criteria is 2.
        // Failure threshold would be 3 - 2 = 1.

        let failure_threshold = n.saturating_sub(self.config.success_threshold);
        if success_count >= self.config.success_threshold {
            // Required success criteria met.
            ctx.reset_children()?;
            Ok(ExecutionStatus::Success)
        } else if failure_count > failure_threshold {
            // Can no longer return success.
            ctx.reset_children()?;
            Ok(ExecutionStatus::Failure)
        } else {
            // Still undecided
            Ok(ExecutionStatus::Running)
        }
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        self.config.load_node_config(config)
    }
    fn static_type() -> NodeType {
        "parallel".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }

    fn reset(&mut self) {
        self.statuses.fill(ExecutionStatus::Running);
    }
}

#[cfg(feature = "betula_editor")]
pub mod ui_support {
    use super::*;
    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for ParallelNode {
        fn ui_title(&self) -> String {
            "parallel 🔀".to_owned()
        }

        fn ui_config(&mut self, ctx: &dyn UiNodeContext, ui: &mut egui::Ui) -> UiConfigResponse {
            let children_count = ctx.children_count();
            let mut ui_response = UiConfigResponse::UnChanged;

            if self.config.success_threshold > children_count {
                self.config.success_threshold = children_count;
                ui_response = UiConfigResponse::Changed;
            }
            ui.horizontal(|ui| {
                let r = ui.checkbox(&mut self.config.memory, "Memory");
                let r = r.on_hover_text("Check this to continue execution where the previous cycle returned, if false the node is reactive and resets each cycle");
                if r.changed() {
                    ui_response = UiConfigResponse::Changed;
                }

                ui.label("Threshold: ");
                let r = ui.add(
                    egui::DragValue::new(&mut self.config.success_threshold)
                    .clamp_range(0..=children_count)
                    .update_while_editing(false),
                );

                if r.changed() {
                    ui_response = UiConfigResponse::Changed;
                }
            });

            ui_response
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("control".to_owned()),
                UiNodeCategory::Name("parallel".to_owned()),
            ]
        }
    }
}
