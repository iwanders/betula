use betula_core::node_prelude::*;

use crate::{enigo, EnigoTokens};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CursorScannerNodeConfig {
    pub a: f64,
    pub b: f64,
    pub speed: f64,
    pub min_radius: f64,
    pub max_radius: f64,
    pub x: f64,
    pub y: f64,

    #[serde(default)]
    pub path_velocity: bool,

    pub interval: f64,
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

            path_velocity: false,
        }
    }
}

use crate::util::Spiral;

#[derive(Default)]
pub struct CursorScannerNode {
    pub config: CursorScannerNodeConfig,

    time: Input<f64>,
    tokens: Output<EnigoTokens>,

    spiral: Option<Spiral>,
    last_time: f64,
}

impl CursorScannerNode {
    pub fn set_center(&mut self, x: f64, y: f64) {
        if let Some(v) = self.spiral.as_mut() {
            v.x = x;
            v.y = y;
        }
    }

    pub fn should_run(&self) -> Result<bool, NodeError> {
        let time = self.time.get()?;
        let should_wait = time < (self.last_time + self.config.interval);
        Ok(!should_wait)
    }
}

impl std::fmt::Debug for CursorScannerNode {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "CursorScannerNode")
    }
}

impl Node for CursorScannerNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        if !self.should_run()? {
            return Ok(ExecutionStatus::Running);
        }

        let time = self.time.get()?;
        if self.spiral.is_none() {
            let spiral = Spiral {
                x: self.config.x,
                y: self.config.y,
                a: self.config.a,
                b: self.config.b,
                speed: self.config.speed,
                max_radius: self.config.max_radius,
                min_radius: self.config.min_radius,
                min_radius_dt: 0.01,
                parameter: 0.0,
                path_velocity: self.config.path_velocity,
            }
            .initialised();
            self.spiral = Some(spiral);
            self.last_time = time;
        }

        let spiral_mut = self.spiral.as_mut().unwrap();
        let dt = time - self.last_time;

        let (x, y) = spiral_mut.advance(dt);

        self.last_time = time;

        let tokens = vec![enigo::agent::Token::MoveMouse(
            x as i32,
            y as i32,
            enigo::Coordinate::Abs,
        )];

        self.tokens.set(EnigoTokens(tokens))?;

        if ctx.children() == 1 {
            let _ = ctx.run(0)?;
        }

        Ok(ExecutionStatus::Running)
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![
            Port::input::<f64>("time"),
            Port::output::<EnigoTokens>("tokens"),
        ])
    }
    fn setup_inputs(
        &mut self,
        interface: &mut dyn BlackboardInputInterface,
    ) -> Result<(), NodeError> {
        self.time = interface.input::<f64>("time")?;
        Ok(())
    }

    fn setup_outputs(
        &mut self,
        interface: &mut dyn BlackboardOutputInterface,
    ) -> Result<(), NodeError> {
        self.tokens = interface.output::<EnigoTokens>("tokens", Default::default())?;
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

                    let r = ui.checkbox(&mut self.config.path_velocity, "Path Vel");
                    modified |= r
                        .on_hover_text("If checked, velocity is along path instead of arc.")
                        .changed();
                });

                ui.horizontal(|ui| {
                    ui.label("speed");
                    let r = ui.add(
                        egui::DragValue::new(&mut self.config.speed)
                            .speed(0.1)
                            .clamp_range(0.0..=1000.0),
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
            0..1
        }
    }
}
