use betula_core::prelude::*;
use betula_core::{BlackboardInterface, Consumer, DirectionalPort, Node, NodeError, NodeStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DelayNode {
    #[serde(skip)]
    time: Consumer<f64>,
    last_time: f64,
    interval: f64,
}

impl DelayNode {
    pub fn new(interval: f64) -> Self {
        DelayNode {
            interval,
            ..Default::default()
        }
    }
}

impl Node for DelayNode {
    fn tick(&mut self, _ctx: &dyn RunContext) -> Result<NodeStatus, NodeError> {
        let time = self.time.get()?;
        if time < (self.last_time + self.interval) {
            return Ok(NodeStatus::Running);
        }
        self.last_time = time;
        Ok(NodeStatus::Success)
    }
    fn ports(&self) -> Result<Vec<DirectionalPort>, NodeError> {
        Ok(vec![DirectionalPort::consumer::<f64>("time")])
    }
    fn setup(
        &mut self,
        port: &DirectionalPort,
        ctx: &mut dyn BlackboardInterface,
    ) -> Result<(), NodeError> {
        let z = ctx.consumes::<f64>(port.name())?;
        self.time = z;
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
        let time = bb.provides::<f64>("time", 0.0)?;
        time.set(1.0)?;

        let root = tree.add_node_boxed(NodeId(Uuid::new_v4()), Box::new(DelayNode::new(5.0)))?;
        let ports = tree.node_mut(root).ok_or("node not found")?.ports()?;
        for p in ports {
            tree.node_mut(root)
                .ok_or("node not found")?
                .setup(&p, &mut bb)?;
        }
        assert!(tree.execute(root)? == NodeStatus::Running);
        time.set(2.0)?;
        assert!(tree.execute(root)? == NodeStatus::Running);
        time.set(6.0)?;
        assert!(tree.execute(root)? == NodeStatus::Success);
        assert!(tree.execute(root)? == NodeStatus::Running);
        time.set(7.0)?;
        assert!(tree.execute(root)? == NodeStatus::Running);
        time.set(10.0)?;
        assert!(tree.execute(root)? == NodeStatus::Running);
        time.set(11.0)?;
        assert!(tree.execute(root)? == NodeStatus::Success);
        assert!(tree.execute(root)? == NodeStatus::Running);
        Ok(())
    }
}
