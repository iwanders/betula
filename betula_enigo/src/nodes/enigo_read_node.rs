use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

use crate::{EnigoBlackboard, EnigoTokens};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnigoReadNodeConfig {
    execute_async: bool,
    #[serde(default)]
    print_tokens: bool,
    #[serde(default)]
    dry_run: bool,
}
impl Default for EnigoReadNodeConfig {
    fn default() -> Self {
        Self {
            execute_async: true,
            print_tokens: false,
            dry_run: false,
        }
    }
}

impl IsNodeConfig for EnigoReadNodeConfig {}

#[derive(Debug, Default)]
pub struct EnigoReadNode {
    tokens: Input<EnigoTokens>,
    input: Input<EnigoBlackboard>,
    pub config: EnigoReadNodeConfig,
}

impl EnigoReadNode {
    pub fn new() -> Self {
        EnigoReadNode::default()
    }
}

impl Node for EnigoReadNode {
    fn execute(&mut self, _ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        let interface = self.input.get()?;
        let tokens = self.tokens.get()?;
        if self.config.print_tokens {
            for (i, t) in tokens.0.iter().enumerate() {
                println!("{i} {t:?}");
            }
        }
        if !self.config.dry_run {
            if self.config.execute_async {
                interface.execute_async(&tokens.0)?;
            } else {
                interface.execute(&tokens.0)?;
            }
        }
        Ok(ExecutionStatus::Success)
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![
            Port::input::<EnigoBlackboard>("enigo"),
            Port::input::<EnigoTokens>("tokens"),
        ])
    }
    fn setup_inputs(
        &mut self,
        interface: &mut dyn BlackboardInputInterface,
    ) -> Result<(), NodeError> {
        self.input = interface.input::<EnigoBlackboard>("enigo")?;
        self.tokens = interface.input::<EnigoTokens>("tokens")?;
        Ok(())
    }

    fn static_type() -> NodeType {
        "enigo_read_node".into()
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

#[cfg(feature = "betula_editor")]
mod ui_support {
    use super::*;
    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for EnigoReadNode {
        fn ui_icon(&self, ui: &mut egui::Ui, desired_size: egui::Vec2) {
            let _ = desired_size;
            ui.add(egui::Label::new("🕹").selectable(false));
        }

        fn ui_config(&mut self, ctx: &dyn UiNodeContext, ui: &mut egui::Ui) -> UiConfigResponse {
            let _ = (ctx);
            let mut modified = false;

            let r = ui.checkbox(&mut self.config.dry_run, "Dry");
            modified |= r
                .on_hover_text("Discard the tokens instead of sending them.")
                .changed();

            let r = ui.checkbox(&mut self.config.print_tokens, "Print");
            modified |= r.on_hover_text("Print tokens using println!").changed();

            let r = ui.checkbox(&mut self.config.execute_async, "Async");
            modified |= r
                .on_hover_text(
                    "Send tokens to background thread instead of blocking until completion.",
                )
                .changed();

            if modified {
                UiConfigResponse::Changed
            } else {
                UiConfigResponse::UnChanged
            }
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("action".to_owned()),
                UiNodeCategory::Name("enigo_read".to_owned()),
            ]
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..0
        }
    }
}
