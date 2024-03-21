use egui::emath::TSTransform;

use betula_core::prelude::*;
use betula_core::NodeId;

#[derive(PartialEq, Clone)]
struct TreeNode {
    id: NodeId,
    children: Vec<NodeId>,
    type_name: String,

    position: egui::Pos2,
}

#[derive(Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TreeView {
    transform: TSTransform,
    drag_value: f32,

    nodes: std::collections::HashMap<NodeId, TreeNode>,
}

impl Eq for TreeView {}

impl TreeView {
    pub fn update(&mut self, tree: &betula_core::basic::BasicTree) {
        self.nodes.clear();
        for id in tree.nodes() {
            let children = tree.children(id).unwrap();
            let node = tree.node_ref(id).unwrap();
            let l = node.borrow();
            use std::ops::Deref;
            let type_name: String = (((*l).deref()).type_name()).to_string();
            let n = TreeNode {
                id,
                type_name,
                children,
                position: egui::Pos2::new(0.0, 120.0),
            };
            self.nodes.insert(id, n);
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
        let (response, painter) = ui.allocate_painter(
            egui::Vec2::new(ui.available_width(), 300.0),
            egui::Sense::hover(),
        );
        let mut lines = vec![];

        for (k, node) in self.nodes.iter_mut() {
            // let this_id = ;
            // let mut area = egui::Area::new(id.with(("subarea", k)));
            // let window_response = ui.ctx().id(this_id).current_pos(current_window_pos);

            let new_window_pos = ui.ctx().memory(|mem| {
                mem.area_rect(id.with(("subarea", k)))
                    .map(|rect| rect.center())
                    .unwrap_or(egui::Pos2::new(0.0, 120.0))
            });
            node.position = new_window_pos;
            println!("new_window_pos: {new_window_pos:?}");
        }

        for (k, node) in self.nodes.iter() {
            let this_id = id.with(("subarea", k));
            let mut area = egui::Area::new(this_id);
            // node.position = area.current_pos();
            // current_pos
            let id = area
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
                            ui.add(Box::new(|ui: &mut egui::Ui| {
                                ui.button(node.type_name.clone())
                            }));

                            for child_id in node.children.iter() {
                                let stroke =
                                    egui::Stroke::new(1.0, egui::Color32::from_rgb(25, 200, 100));
                                let fill =
                                    egui::Color32::from_rgb(50, 100, 150).linear_multiply(0.25);
                                let mut points = [
                                    node.position,
                                    node.position + egui::Vec2::new(0.0, -5.0),
                                    self.nodes
                                        .get(child_id)
                                        .expect("child should be present")
                                        .position
                                        + egui::Vec2::new(0.0, 5.0),
                                    self.nodes
                                        .get(child_id)
                                        .expect("child should be present")
                                        .position,
                                ];
                                for p in points.iter_mut() {
                                    *p = self.transform * *p;
                                }
                                println!("Points: {points:?}");
                                let shape = egui::epaint::CubicBezierShape::from_points_stroke(
                                    points, false, fill, stroke,
                                );
                                lines.push(egui::Shape::CubicBezier(shape));
                            }
                        });
                })
                .response
                .layer_id;

            ui.ctx().set_transform_layer(id, transform);
            // println!("transform: {transform:?}");
        }

        ui.painter().add(egui::Shape::Vec(lines));
    }
}
