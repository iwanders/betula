use betula_core::prelude::*;
use betula_core::{ExecutionStatus, Node, NodeError, NodeType};

/// Node that executes nodes in sequence until one does not return [`ExecutionStatus::Success`].
///
/// Runs nodes from left to right, ignoring [`ExecutionStatus::Success`] but
/// returning the first [`ExecutionStatus::Failure`] or [`ExecutionStatus::Running`]
/// encountered, at this point that value is returned.
/// The node returns [`ExecutionStatus::Success`] if all child nodes succceed.
#[derive(Debug, Copy, Clone, Default)]
pub struct SequenceNode {}
impl Node for SequenceNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        for id in 0..ctx.children() {
            match ctx.run(id)? {
                ExecutionStatus::Success => {}
                other => return Ok(other), // fail or running.
            }
        }

        // All children succeeded.
        Ok(ExecutionStatus::Success)
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
    use betula_editor::{UiNode, UiNodeCategory};

    impl UiNode for SequenceNode {
        fn ui_title(&self) -> String {
            "sequence ⮊".to_owned()
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
