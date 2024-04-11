use betula_core::node_prelude::*;

#[derive(Debug, Default)]
pub struct TimeNode {
    time_output: Output<f64>,
}
impl TimeNode {
    pub fn new() -> Self {
        TimeNode::default()
    }
}

impl Node for TimeNode {
    fn tick(&mut self, _ctx: &dyn RunContext) -> Result<NodeStatus, NodeError> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let start = SystemTime::now();
        let since_the_epoch = start.duration_since(UNIX_EPOCH)?;
        self.time_output.set(since_the_epoch.as_secs_f64())?;
        Ok(NodeStatus::Success)
    }
    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![Port::output::<f64>("time")])
    }
    fn setup_outputs(
        &mut self,
        interface: &mut dyn BlackboardOutputInterface,
    ) -> Result<(), NodeError> {
        self.time_output = interface.output::<f64>("time", 0.0)?;
        Ok(())
    }

    fn static_type() -> NodeType
    where
        Self: Sized,
    {
        "common_time".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_blackboard_reqs() -> Result<(), NodeError> {
        use betula_core::{
            basic::{BasicBlackboard, BasicTree},
            NodeId,
        };
        use uuid::Uuid;
        let mut tree = BasicTree::new();
        let mut bb = BasicBlackboard::default();
        let root = tree.add_node_boxed(NodeId(Uuid::new_v4()), Box::new(TimeNode::default()))?;
        let ports = tree.node_mut(root).ok_or("node not found")?.ports()?;
        assert!(ports.len() == 1);
        tree.node_mut(root)
            .ok_or("node not found")?
            .setup_outputs(&mut bb)?;
        tree.execute(root)?;

        let v = bb.input::<f64>("time")?;
        assert!(v.get()? != 0.0);
        println!("time: {v:?} -> {}", v.get()?);
        Ok(())
    }
}
