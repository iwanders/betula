use betula_core::node_prelude::*;

use crate::{CursorPosition, EnigoBlackboard};

#[derive(Debug, Default)]
pub struct EnigoCursorNode {
    input: Input<EnigoBlackboard>,
    output: Output<CursorPosition>,
}

impl EnigoCursorNode {
    pub fn new() -> Self {
        EnigoCursorNode::default()
    }
}

impl Node for EnigoCursorNode {
    fn execute(&mut self, _ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        let interface = self.input.get()?;
        let pos = interface.cursor_location()?;

        self.output.set(pos)?;
        Ok(ExecutionStatus::Success)
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![
            Port::input::<EnigoBlackboard>("enigo"),
            Port::output::<CursorPosition>("cursor"),
        ])
    }
    fn setup_inputs(
        &mut self,
        interface: &mut dyn BlackboardInputInterface,
    ) -> Result<(), NodeError> {
        self.input = interface.input::<EnigoBlackboard>("enigo")?;
        Ok(())
    }

    fn setup_outputs(
        &mut self,
        interface: &mut dyn BlackboardOutputInterface,
    ) -> Result<(), NodeError> {
        self.output = interface.output::<CursorPosition>("cursor", Default::default())?;
        Ok(())
    }

    fn static_type() -> NodeType {
        "enigo_cursor".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}

#[cfg(feature = "betula_editor")]
mod ui_support {
    use super::*;
    use betula_editor::{egui, UiNode, UiNodeCategory};

    impl UiNode for EnigoCursorNode {
        fn ui_title(&self) -> String {
            "cursor".to_owned()
        }

        fn ui_icon(&self, ui: &mut egui::Ui, desired_size: egui::Vec2) {
            let visuals = ui.style().noninteractive();
            let svg_paths = betula_editor::widgets::SVGPaths{
                viewbox: egui::vec2(130.0, 130.0), // was 100, this adds some whitespace.
                transform: egui::vec2(20.0, -167.0), // was 0.0, -197.0
                paths: vec![
                    "m 43.666015,289.96634 -20.433078,-35.39113 -22.68307393,22.68307 -2.75e-6,-79.5916 68.92834368,39.79581 -30.985656,8.30258 20.433077,35.39113 z".to_owned(),
                ],
                fill: egui::Color32::TRANSPARENT,
                stroke: visuals.fg_stroke,
            };
            ui.add(svg_paths.to_widget(desired_size));
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("provider".to_owned()),
                UiNodeCategory::Name("enigo_cursor".to_owned()),
            ]
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..0
        }
    }
}
