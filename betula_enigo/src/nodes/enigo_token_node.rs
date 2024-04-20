use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

use crate::{EnigoBlackboard, EnigoRunner};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EnigoTokenNodeConfig {
    execute_async: bool,
}
impl IsNodeConfig for EnigoTokenNodeConfig {}

#[derive(Debug, Default)]
pub struct EnigoTokenNode {
    input: Input<EnigoBlackboard>,
    pub config: EnigoTokenNodeConfig,
}

impl EnigoTokenNode {
    pub fn new() -> Self {
        EnigoTokenNode::default()
    }
}

impl Node for EnigoTokenNode {
    fn execute(&mut self, _ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        let mut interface = self.input.get()?;
        use enigo::agent::Token;
        if self.config.execute_async {
            // interface.
        } else {
            interface.execute(&Token::Text("Hello World! ❤️".to_string()))?;
        }
        Ok(ExecutionStatus::Success)
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![Port::input::<EnigoBlackboard>("enigo")])
    }
    fn setup_inputs(
        &mut self,
        interface: &mut dyn BlackboardInputInterface,
    ) -> Result<(), NodeError> {
        self.input = interface.input::<EnigoBlackboard>("enigo")?;
        Ok(())
    }

    fn static_type() -> NodeType {
        "enigo_token".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        self.config.load_node_config(config)
    }
}

#[cfg(feature = "betula_egui")]
mod ui_support {
    use super::*;
    use betula_egui::{egui::Ui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for EnigoTokenNode {
        fn ui_title(&self) -> String {
            "enigo_token 🖱🖮 ".to_owned()
        }

        fn ui_config(
            &mut self,
            ctx: &dyn UiNodeContext,
            ui: &mut Ui,
            _scale: f32,
        ) -> UiConfigResponse {
            let _ = ctx;
            let mut ui_response = UiConfigResponse::UnChanged;
            ui.horizontal(|ui| {
                let r = ui.checkbox(&mut self.config.execute_async, "Async");
                if r.changed() {
                    ui_response = UiConfigResponse::Changed;
                }
            });

            ui_response
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("action".to_owned()),
                UiNodeCategory::Name("enigo".to_owned()),
            ]
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..0
        }
    }
}
