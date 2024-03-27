use betula_core::prelude::*;
use betula_core::{
    BlackboardInterface, Consumer, DirectionalPort, Node, NodeConfig, NodeError, NodeStatus,
    NodeType,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DelayNodeConfig {
    pub interval: f64,
}

#[derive(Debug, Default)]
pub struct DelayNode {
    time: Consumer<f64>,
    last_time: f64,
    config: DelayNodeConfig,
}

impl DelayNode {
    pub fn new(interval: f64) -> Self {
        DelayNode {
            config: DelayNodeConfig { interval },
            ..Default::default()
        }
    }
}

impl Node for DelayNode {
    fn tick(&mut self, _ctx: &dyn RunContext) -> Result<NodeStatus, NodeError> {
        let time = self.time.get()?;
        if time < (self.last_time + self.config.interval) {
            return Ok(NodeStatus::Running);
        }
        self.last_time = time;
        Ok(NodeStatus::Success)
    }

    fn ports(&self) -> Result<Vec<DirectionalPort>, NodeError> {
        Ok(vec![DirectionalPort::consumer::<f64>("time")])
    }

    fn port_setup(
        &mut self,
        port: &DirectionalPort,
        interface: &mut dyn BlackboardInterface,
    ) -> Result<(), NodeError> {
        let z = interface.consumes::<f64>(port.name())?;
        self.time = z;
        Ok(())
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        self.config.load_node_config(config)
    }
    fn node_type(&self) -> NodeType {
        "common_delay".into()
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
                .port_setup(&p, &mut bb)?;
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
