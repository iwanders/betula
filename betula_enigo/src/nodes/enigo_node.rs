use betula_core::node_prelude::*;

use crate::{EnigoBlackboard, EnigoRunner};

#[derive(Debug, Default)]
pub struct EnigoNode {
    needs_creation: bool,
    output: Output<EnigoBlackboard>,
}

impl EnigoNode {
    pub fn new() -> Self {
        EnigoNode::default()
    }
}

impl Node for EnigoNode {
    fn tick(&mut self, _ctx: &dyn RunContext) -> Result<NodeStatus, NodeError> {
        if self.needs_creation {
            let v = EnigoRunner::new()?;
            self.output.set(EnigoBlackboard { interface: Some(v) })?;
            self.needs_creation = false;
        }
        Ok(NodeStatus::Success)
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![Port::output::<EnigoBlackboard>("enigo")])
    }
    fn setup_outputs(
        &mut self,
        interface: &mut dyn BlackboardOutputInterface,
    ) -> Result<(), NodeError> {
        self.output = interface.output::<EnigoBlackboard>("enigo", EnigoBlackboard::default())?;
        Ok(())
    }

    fn static_type() -> NodeType {
        "enigo".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}
