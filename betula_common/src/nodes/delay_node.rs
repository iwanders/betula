use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DelayNodeConfig {
    /// The interval to wait between executions in 'time' unit.
    pub interval: f64,
}
impl IsNodeConfig for DelayNodeConfig {}

/// Node to delay execution of its children.
///
/// Returns [`ExecutionStatus::Running`] while the interval since the last
/// execution is not yet exceeded. When the interval is exceeded it runs
/// its one child node and returns its status. If there is no child node
/// to be executed it returns [`ExecutionStatus::Running`].
///
/// One input port `time`, of type `f64`, which usually is time in seconds.
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
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        let time = self.time.get()?;
        if time < (self.last_time + self.config.interval) {
            return Ok(ExecutionStatus::Running);
        }
        self.last_time = time;

        if ctx.children() == 1 {
            ctx.run(0)
        } else {
            Ok(ExecutionStatus::Success)
        }
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![Port::input::<f64>("time")])
    }

    fn setup_inputs(
        &mut self,
        interface: &mut dyn BlackboardInputInterface,
    ) -> Result<(), NodeError> {
        self.time = interface.input::<f64>("time")?;
        Ok(())
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        self.config.load_node_config(config)
    }
    fn static_type() -> NodeType {
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
        let time = bb.output::<f64>("time", 0.0)?;
        time.set(1.0)?;

        let root = tree.add_node_boxed(NodeId(Uuid::new_v4()), Box::new(DelayNode::new(5.0)))?;
        let ports = tree.node_mut(root).ok_or("node not found")?.ports()?;
        assert!(ports.len() == 1);
        tree.node_mut(root)
            .ok_or("node not found")?
            .setup_inputs(&mut bb)?;
        assert!(tree.execute(root)? == ExecutionStatus::Running);
        time.set(2.0)?;
        assert!(tree.execute(root)? == ExecutionStatus::Running);
        time.set(6.0)?;
        assert!(tree.execute(root)? == ExecutionStatus::Success);
        assert!(tree.execute(root)? == ExecutionStatus::Running);
        time.set(7.0)?;
        assert!(tree.execute(root)? == ExecutionStatus::Running);
        time.set(10.0)?;
        assert!(tree.execute(root)? == ExecutionStatus::Running);
        time.set(11.0)?;
        assert!(tree.execute(root)? == ExecutionStatus::Success);
        assert!(tree.execute(root)? == ExecutionStatus::Running);
        Ok(())
    }
}
