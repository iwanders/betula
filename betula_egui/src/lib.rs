use egui::emath::TSTransform;

use betula_core::NodeId;

#[derive(PartialEq, Clone)]
struct TreeNode {
    id: NodeId,
    children: Vec<NodeId>,

    position: egui::Pos2,
}

#[derive(Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TreeView {
    transform: TSTransform,
    drag_value: f32,

    nodes: Vec<TreeNode>,
}

impl Eq for TreeView {}

impl TreeView {
    pub fn update(&mut self, tree: &dyn betula_core::Tree) {
        self.nodes.clear();
        for id in tree.nodes() {
            let n = TreeNode {
                id,
                children: tree.children(id),
                position: egui::Pos2::new(0.0, 120.0),
            };
            self.nodes.push(n);
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let (id, rect) = ui.allocate_space(ui.available_size());
        let response = ui.interact(rect, id, egui::Sense::click_and_drag());
        // Allow dragging the background as well.
        if response.dragged() {
            self.transform.translation += response.drag_delta();
        }

        // Plot-like reset
        if response.double_clicked() {
            self.transform = TSTransform::default();
        }

        let transform =
            TSTransform::from_translation(ui.min_rect().left_top().to_vec2()) * self.transform;

        if let Some(pointer) = ui.ctx().input(|i| i.pointer.hover_pos()) {
            // Note: doesn't catch zooming / panning if a button in this PanZoom container is hovered.
            if response.hovered() {
                let pointer_in_layer = transform.inverse() * pointer;
                let zoom_delta = ui.ctx().input(|i| i.zoom_delta());
                let pan_delta = ui.ctx().input(|i| i.smooth_scroll_delta);

                // Zoom in on pointer:
                self.transform = self.transform
                    * TSTransform::from_translation(pointer_in_layer.to_vec2())
                    * TSTransform::from_scaling(zoom_delta)
                    * TSTransform::from_translation(-pointer_in_layer.to_vec2());

                // Pan:
                self.transform = TSTransform::from_translation(pan_delta) * self.transform;
            }
        }

        for (i, node) in self.nodes.iter().enumerate() {
            let id = egui::Area::new(id.with(("subarea", i)))
                .default_pos(node.position)
                // Need to cover up the pan_zoom demo window,
                // but may also cover over other windows.
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    ui.set_clip_rect(transform.inverse() * rect);
                    egui::Frame::default()
                        .rounding(egui::Rounding::same(4.0))
                        .inner_margin(egui::Margin::same(8.0))
                        .stroke(ui.ctx().style().visuals.window_stroke)
                        .fill(ui.style().visuals.panel_fill)
                        .show(ui, |ui| {
                            ui.style_mut().wrap = Some(false);
                            // callback(ui, self)
                            ui.add(Box::new(|ui: &mut egui::Ui| ui.button("right top ):")))
                        });
                })
                .response
                .layer_id;
            ui.ctx().set_transform_layer(id, transform);
        }
    }
}
