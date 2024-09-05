use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StringWriteNodeConfig {
    pub text: String,
}
impl IsNodeConfig for StringWriteNodeConfig {}

/// Node to write a static string.
///
/// Always succeeds and writes the fixed string to the blackboard.
///
/// One output port `text`, of type `string`.
#[derive(Debug, Default)]
pub struct StringWriteNode {
    text: Output<String>,
    pub config: StringWriteNodeConfig,
}

impl Node for StringWriteNode {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        let _ = ctx;
        self.text.set(self.config.text.clone())?;
        Ok(ExecutionStatus::Success)
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![Port::output::<String>("text")])
    }

    fn setup_outputs(
        &mut self,
        interface: &mut dyn BlackboardOutputInterface,
    ) -> Result<(), NodeError> {
        self.text = interface.output::<String>("text", "".to_owned())?;
        Ok(())
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        self.config.load_node_config(config)
    }

    fn reset(&mut self) {}

    fn static_type() -> NodeType {
        "std_string_write".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}

#[cfg(feature = "betula_editor")]
pub mod ui_support {
    use super::*;
    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for StringWriteNode {
        fn ui_title(&self) -> String {
            "string".to_owned()
        }
        fn ui_icon(&self, ui: &mut egui::Ui, desired_size: egui::Vec2) {
            let _ = desired_size;
            ui.add(egui::Label::new("✏").selectable(false));
        }

        fn ui_config(
            &mut self,
            ctx: &dyn UiNodeContext,
            ui: &mut egui::Ui,
            _scale: f32,
        ) -> UiConfigResponse {
            let _ = ctx;
            let mut ui_response = UiConfigResponse::UnChanged;
            ui.horizontal(|ui| {
                ui.label("Text: ");
                let r = ui.add(egui::TextEdit::singleline(&mut self.config.text));
                if r.changed() {
                    // println!("Changed! now: {}", self.config.interval);
                    ui_response = UiConfigResponse::Changed;
                }
            });

            ui_response
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..0
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("provider".to_owned()),
                // UiNodeCategory::Group("time".to_owned()),
                UiNodeCategory::Name("string_write".to_owned()),
            ]
        }
    }
}
