use betula_core::node_prelude::*;

use crate::{enigo, EnigoBlackboard};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CursorScannerNodeConfig {
    a: f64,
    b: f64,
    speed: f64,
    min_radius: f64,
    max_radius: f64,
    x: f64,
    y: f64,

    interval: f64,
}
impl IsNodeConfig for CursorScannerNodeConfig {}

impl Default for CursorScannerNodeConfig {
    fn default() -> Self {
        Self {
            a: 0.0,
            b: 10.0,
            max_radius: 200.0,
            min_radius: 30.0,

            x: 1000.0,
            y: 500.0,

            speed: 2.0,
            interval: 0.05,
        }
    }
}

use crate::util::Spiral;

#[derive(Default)]
pub struct CursorScannerNode {
    pub config: CursorScannerNodeConfig,

    enigo: Input<EnigoBlackboard>,
    time: Input<f64>,

    spiral: Option<Spiral>,
    last_time: f64,
}

impl std::fmt::Debug for CursorScannerNode {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "CursorScannerNode")
    }
}

impl Node for CursorScannerNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        let _ = ctx;

        let enigo_instance = self.enigo.get()?;
        let time = self.time.get()?;

        if self.spiral.is_none() {
            let mut spiral = Spiral {
                x: self.config.x,
                y: self.config.y,
                a: self.config.a,
                b: self.config.b,
                speed: self.config.speed,
                max_radius: self.config.max_radius,
                min_radius: self.config.min_radius,
                min_radius_dt: 0.01,
                parameter: 0.0,
            };
            spiral.reset();
            self.spiral = Some(spiral);
            self.last_time = time;
        }

        if time < (self.last_time + self.config.interval) {
            return Ok(ExecutionStatus::Running);
        }

        let spiral_mut = self.spiral.as_mut().unwrap();
        let dt = time - self.last_time;

        let (x, y) = spiral_mut.advance(dt as f64);

        self.last_time = time;

        let tokens = vec![enigo::agent::Token::MoveMouse(
            x as i32,
            y as i32,
            enigo::Coordinate::Abs,
        )];
        enigo_instance.execute_async(&tokens)?;

        return Ok(ExecutionStatus::Running);
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![
            Port::input::<EnigoBlackboard>("enigo"),
            Port::input::<f64>("time"),
        ])
    }
    fn setup_inputs(
        &mut self,
        interface: &mut dyn BlackboardInputInterface,
    ) -> Result<(), NodeError> {
        self.enigo = interface.input::<EnigoBlackboard>("enigo")?;
        self.time = interface.input::<f64>("time")?;
        Ok(())
    }

    fn static_type() -> NodeType {
        "enigo_cursor_scanner".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        self.spiral = None;
        self.config.load_node_config(config)
    }

    fn reset(&mut self) {
        self.spiral = None;
    }
}

#[cfg(feature = "betula_editor")]
mod ui_support {
    use super::*;

    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for CursorScannerNode {
        fn ui_title(&self) -> String {
            "cursor scanner 📻".to_owned()
        }

        fn ui_config(
            &mut self,
            ctx: &dyn UiNodeContext,
            ui: &mut egui::Ui,
            scale: f32,
        ) -> UiConfigResponse {
            let _ = (ctx, scale);

            let mut modified = false;

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("a");
                    let r = ui.add(egui::DragValue::new(&mut self.config.a).speed(0.1));
                    modified |= r.on_hover_text("35.0 does reasonable").changed();
                    ui.label("+");
                    let r = ui.add(egui::DragValue::new(&mut self.config.b).speed(0.1));
                    modified |= r.on_hover_text("10.0 does reasonable").changed();
                    ui.label("* t");
                });

                ui.horizontal(|ui| {
                    ui.label("Radius");
                    let r = ui.add(
                        egui::DragValue::new(&mut self.config.min_radius)
                            .speed(5.0)
                            .clamp_range(0.0..=self.config.max_radius),
                    );
                    modified |= r.changed();
                    ui.label("to");
                    let r = ui.add(
                        egui::DragValue::new(&mut self.config.max_radius)
                            .speed(5.0)
                            .clamp_range(self.config.min_radius..=2000.0),
                    );
                    modified |= r.changed();
                });

                ui.horizontal(|ui| {
                    ui.label("x");
                    let r = ui.add(
                        egui::DragValue::new(&mut self.config.x)
                            .speed(5.0)
                            .clamp_range(0.0..=1920.0),
                    );
                    modified |= r.changed();
                    ui.label("y");
                    let r = ui.add(
                        egui::DragValue::new(&mut self.config.y)
                            .speed(5.0)
                            .clamp_range(0.0..=1080.0),
                    );
                    modified |= r.changed();
                });

                ui.horizontal(|ui| {
                    ui.label("speed");
                    let r = ui.add(
                        egui::DragValue::new(&mut self.config.speed)
                            .speed(0.1)
                            .clamp_range(0.0..=100.0),
                    );
                    modified |= r.changed();
                    ui.label("Interval: ");
                    let r =
                        betula_editor::egui_util::time_drag_value(ui, &mut self.config.interval);
                    modified |= r.changed();
                });
            });
            if modified {
                UiConfigResponse::Changed
            } else {
                UiConfigResponse::UnChanged
            }
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("action".to_owned()),
                UiNodeCategory::Name("enigo_cursor_scanner".to_owned()),
            ]
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..0
        }
    }
}
