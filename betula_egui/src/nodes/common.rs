use crate::UiNode;
use egui::Ui;

use betula_common::nodes;

impl UiNode for nodes::DelayNode {
    fn ui_config(&mut self, ui: &mut Ui, _scale: f32) {
        let r = ui.add(egui::Slider::new(&mut self.config.interval, 0.0..=100.0).text("My value"));
        if r.changed() {
            println!("Changed! now: {}", self.config.interval);
        }
    }
    fn ui_child_range(&self) -> std::ops::Range<usize> {
        0..usize::MAX
    }
}
/*
impl NodeUi for nodes::TimeNode {
    fn name(&self) -> String {
        nodes::TimeNode::static_type().into()
    }
}
*/
