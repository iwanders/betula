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
    use betula_editor::{UiNode, UiNodeCategory};

    impl UiNode for EnigoCursorNode {
        fn ui_title(&self) -> String {
            "enigo cursor 🖱 ".to_owned()
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
