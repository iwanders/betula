use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

use crate::{EnigoBlackboard, EnigoRunner};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EnigoTokenNodeConfig {}
impl IsNodeConfig for EnigoTokenNodeConfig {}

#[derive(Debug, Default)]
pub struct EnigoTokenNode {
    input: Input<EnigoBlackboard>,
}

impl EnigoTokenNode {
    pub fn new() -> Self {
        EnigoTokenNode::default()
    }
}

impl Node for EnigoTokenNode {
    fn tick(&mut self, _ctx: &dyn RunContext) -> Result<NodeStatus, NodeError> {
        let mut interface = self.input.get()?;
        use enigo::agent::Token;
        interface.execute(&Token::Text("Hello World! ❤️".to_string()))?;
        Ok(NodeStatus::Success)
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
}

#[cfg(feature = "betula_egui")]
mod ui_support {
    use super::*;
    use betula_egui::{UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl UiNode for EnigoTokenNode {
        fn ui_title(&self) -> String {
            "enigo_token ".to_owned()
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            vec![
                UiNodeCategory::Folder("action".to_owned()),
                UiNodeCategory::Name("enigo".to_owned()),
            ]
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            0..0 // todo without this we encounter an unreachable in the ui!
        }
    }
}
