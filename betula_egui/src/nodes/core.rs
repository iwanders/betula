use crate::NodeUi;
use betula_core::{nodes, Node, NodeType};

impl NodeUi for nodes::SequenceNode {
    fn name(&self) -> String {
        nodes::SequenceNode::static_type().into()
    }
}

impl NodeUi for nodes::SuccessNode {
    fn name(&self) -> String {
        nodes::SuccessNode::static_type().into()
    }
}

impl NodeUi for nodes::SelectorNode {
    fn name(&self) -> String {
        nodes::SelectorNode::static_type().into()
    }
}

impl NodeUi for nodes::FailureNode {
    fn name(&self) -> String {
        nodes::FailureNode::static_type().into()
    }
}
