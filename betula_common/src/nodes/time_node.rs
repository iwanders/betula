use betula_core::prelude::*;
use betula_core::{BlackboardInterface, DirectionalPort, Node, NodeError, NodeStatus, Provider};

#[derive(Debug, Default)]
pub struct TimeNode {
    time_provider: Provider<f64>,
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
        self.time_provider.set(since_the_epoch.as_secs_f64())?;
        Ok(NodeStatus::Success)
    }
    fn ports(&self) -> Result<Vec<DirectionalPort>, NodeError> {
        Ok(vec![DirectionalPort::provider::<f64>("time")])
    }
    fn setup(
        &mut self,
        port: &DirectionalPort,
        ctx: &mut dyn BlackboardInterface,
    ) -> Result<(), NodeError> {
        let z = ctx.provides::<f64>(port.name(), 0.0)?;
        self.time_provider = z;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_blackboard_reqs() -> Result<(), NodeError> {
        use betula_core::{
            basic::{BasicBlackboard, BasicTree},
            NodeId, Uuid,
        };
        let mut tree = BasicTree::new();
        let mut bb = BasicBlackboard::default();
        let root = tree.add_node_boxed(NodeId(Uuid::new_v4()), Box::new(TimeNode::default()))?;
        let ports = tree.node_mut(root).ok_or("node not found")?.ports()?;
        for p in ports {
            tree.node_mut(root)
                .ok_or("node not found")?
                .setup(&p, &mut bb)?;
        }
        tree.execute(root)?;

        let v = bb.consumes::<f64>("time")?;
        assert!(v.get()? != 0.0);
        println!("time: {v:?} -> {}", v.get()?);
        Ok(())
    }
}
