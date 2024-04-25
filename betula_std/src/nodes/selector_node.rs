use betula_core::prelude::*;
use betula_core::{ExecutionStatus, Node, NodeError, NodeType};

/// Node that executes nodes in sequence returning the first non-[`ExecutionStatus::Failure`].
///
/// Runs nodes from left to right, ignoring [`ExecutionStatus::Failure`] but
/// returning the first [`ExecutionStatus::Success`] or [`ExecutionStatus::Running`]
/// encountered, at this point that value is returned.
/// The node returns [`ExecutionStatus::Failure`] if all child nodes failed.
#[derive(Debug, Copy, Clone, Default)]
pub struct SelectorNode {}
impl Node for SelectorNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        for id in 0..ctx.children() {
            match ctx.run(id)? {
                ExecutionStatus::Failure => {}
                other => return Ok(other),
            }
        }

        // Reached here, all children must've failed.
        Ok(ExecutionStatus::Failure)
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
    use betula_editor::{UiNode, UiNodeCategory};

    impl UiNode for SelectorNode {
        fn ui_title(&self) -> String {
            "selector ⛶".to_owned()
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
        let root = tree.add_node_boxed(NodeId(Uuid::new_v4()), Box::new(SelectorNode {}))?;
        let f1 = tree.add_node_boxed(NodeId(Uuid::new_v4()), Box::new(FailureNode {}))?;
        let s1 = tree.add_node_boxed(NodeId(Uuid::new_v4()), Box::new(SuccessNode {}))?;
        tree.set_children(root, &vec![f1, s1])?;
        let res = tree.execute(root)?;
        assert_eq!(res, ExecutionStatus::Success);
        Ok(())
    }
}
