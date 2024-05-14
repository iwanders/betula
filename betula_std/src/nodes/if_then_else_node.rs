use betula_core::node_prelude::*;

/// Node to do an if(then else) statement.
///
/// The node will execute the first child, if the status is [`ExecutionStatus::Running`] it will
/// return running.
/// If the first child returns [`ExecutionStatus::Success`], the second child is executed and its
/// status returned, if the first child returns  [`ExecutionStatus::Failure`], if the third child
/// exists, it it executed and its status is returned, else it returns [`ExecutionStatus::Failure`].
#[derive(Debug, Default)]
pub struct IfThenElseNode {}

impl Node for IfThenElseNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        if ctx.children() < 2 || ctx.children() > 3 {
            return Err("IfThenElseNode must have two or three child nodes".into());
        }

        let r = match ctx.run(0)? {
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
        if r != ExecutionStatus::Running {
            for i in 0..ctx.children() {
                ctx.reset_recursive(i)?;
            }
        }
        Ok(r)
    }

    fn static_type() -> NodeType {
        "std_if_then_else".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}

#[cfg(feature = "betula_editor")]
pub mod ui_support {
    use super::*;
    use betula_editor::{egui, UiNode, UiNodeCategory};

    impl UiNode for IfThenElseNode {
        fn ui_title(&self) -> String {
            "if".to_owned()
        }
        fn ui_icon(&self, ui: &mut egui::Ui, desired_size: egui::Vec2) {
            let _ = desired_size;
            ui.add(egui::Label::new("?").selectable(false));
        }

        fn ui_child_range(&self) -> std::ops::Range<usize> {
            2..3
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("control".to_owned()),
                // UiNodeCategory::Group("time".to_owned()),
                UiNodeCategory::Name("if_then_else".to_owned()),
            ]
        }
    }
}
