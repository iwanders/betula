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

    fn ui_config(&self, node: &mut ViewerNode, ui: &mut Ui, _scale: f32) {
        let r = ui.add(egui::Slider::new(&mut node.my_f32, 0.0..=100.0).text("My value"));
        if r.changed() {
            println!("Changed! now: {}", node.my_f32);
        }
    }
}

impl NodeUi for nodes::TimeNode {
    fn name(&self) -> String {
        nodes::TimeNode::static_type().into()
    }
}
