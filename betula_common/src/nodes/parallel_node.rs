use betula_core::node_prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ParallelNodeConfig {
    pub success_threshold: usize,
}
impl IsNodeConfig for ParallelNodeConfig {}

#[derive(Debug, Default)]
pub struct ParallelNode {
    pub config: ParallelNodeConfig,
}

impl ParallelNode {
    pub fn new(success_threshold: usize) -> Self {
        ParallelNode {
            config: ParallelNodeConfig { success_threshold },
        }
    }
}

impl Node for ParallelNode {
    fn tick(&mut self, ctx: &dyn RunContext) -> Result<NodeStatus, NodeError> {
        let mut success_count = 0;
        let mut failure_count = 0;
        let n = ctx.children();
        for id in 0..n {
            match ctx.run(id)? {
                NodeStatus::Success => success_count += 1,
                NodeStatus::Failure => failure_count += 1,
                NodeStatus::Running => {}
            }
        }

        // Lets say three nodes.
        // Success criteria is 2.
        // Failure threshold would be 3 - 2 = 1.

        let failure_threshold = n.saturating_sub(self.config.success_threshold);
        if success_count >= self.config.success_threshold {
            // Required success criteria met.
            Ok(NodeStatus::Success)
        } else if failure_count > failure_threshold {
            // Can no longer return success.
            Ok(NodeStatus::Failure)
        } else {
            // Still undecided
            Ok(NodeStatus::Running)
        }
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        self.config.load_node_config(config)
    }
    fn static_type() -> NodeType {
        "common_parallel".into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}
