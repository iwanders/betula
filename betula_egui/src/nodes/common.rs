use crate::{NodeUi, ViewerNode};
use betula_core::{Node, NodeConfig};
use egui::{widgets, Ui};

use betula_common::nodes;

impl NodeUi for nodes::DelayNode {
    fn name(&self) -> String {
        nodes::DelayNode::static_type().into()
    }
    fn has_config(&self, _node: &ViewerNode) -> bool {
        true
    }
    fn ui_config(&self, _node: &mut ViewerNode, _ui: &mut Ui, _scale: f32) {}
}

impl NodeUi for nodes::TimeNode {
    fn name(&self) -> String {
        nodes::TimeNode::static_type().into()
    }
}
