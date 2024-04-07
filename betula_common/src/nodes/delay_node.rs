use betula_core::prelude::*;
use betula_core::{
    BlackboardInterface, Input, Node, NodeConfig, NodeError, NodeStatus, NodeType, Port,
    PortDirection, PortName,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DelayNodeConfig {
    pub interval: f64,
}

#[derive(Debug, Default)]
pub struct DelayNode {
    time: Input<f64>,
    last_time: f64,
    pub config: DelayNodeConfig,
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
    fn tick(&mut self, ctx: &dyn RunContext) -> Result<NodeStatus, NodeError> {
        let time = self.time.get()?;
        if time < (self.last_time + self.config.interval) {
            return Ok(NodeStatus::Running);
        }
        self.last_time = time;

        if ctx.children() == 1 {
            ctx.run(0)
        } else {
            Ok(NodeStatus::Success)
        }
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![Port::input::<f64>("time")])
    }

    fn port_setup(
        &mut self,
        port: &PortName,
        direction: PortDirection,
        interface: &mut dyn BlackboardInterface,
    ) -> Result<(), NodeError> {
        let _ = direction;
        self.time = interface.input::<f64>(port)?;
        Ok(())
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        self.config.load_node_config(config)
    }
    fn static_type() -> NodeType
    where
        Self: Sized,
    {
        "common_delay".into()
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
        let time = bb.output::<f64>(&"time".into(), 0.0)?;
        time.set(1.0)?;

        let root = tree.add_node_boxed(NodeId(Uuid::new_v4()), Box::new(DelayNode::new(5.0)))?;
        let ports = tree.node_mut(root).ok_or("node not found")?.ports()?;
        for p in ports {
            tree.node_mut(root).ok_or("node not found")?.port_setup(
                &p.name(),
                PortDirection::Input,
                &mut bb,
            )?;
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
