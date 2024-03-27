// use betula_core::prelude::*;
use betula_core::{BetulaError, NodeId, NodeType};
use serde::{Deserialize, Serialize};

use crate::support::{
    ConfigConverter, DefaultConfigConverter, DefaultConfigRequirements, DefaultFactory,
    DefaultFactoryRequirements, NodeFactory,
};

// Only really used internally as an type that implements Serialize and
// Deserialize.
type NodeConfig = serde_json::Value;

#[derive(Serialize, Deserialize, Debug)]
struct TreeNode {
    id: NodeId,
    node_type: String,
    config: Option<NodeConfig>,
    children: Vec<NodeId>,
}

#[derive(Serialize, Deserialize, Debug)]
struct TreeConfig {
    nodes: Vec<TreeNode>,
}

use std::collections::HashMap;
#[derive(Debug)]
struct TypeSupport {
    factory: Box<dyn NodeFactory>,
    config_converter: Option<Box<dyn ConfigConverter>>,
}
#[derive(Debug, Default)]
pub struct TreeLoader {
    support: HashMap<NodeType, TypeSupport>,
}
impl TreeLoader {
    pub fn new() -> Self {
        TreeLoader::default()
    }

    pub fn add_factory(&mut self, node_type: NodeType, factory: Box<dyn NodeFactory>) {
        let entry = self.support.insert(
            node_type,
            TypeSupport {
                factory,
                config_converter: None,
            },
        );
        if let Some(old_entry) = entry {
            if let Some(config_converter) = old_entry.config_converter {
                self.add_config_converter(&node_type, config_converter)
                    .unwrap();
            }
        }
    }

    pub fn add_config_converter(
        &mut self,
        node_type: &NodeType,
        config_converter: Box<dyn ConfigConverter>,
    ) -> Result<(), BetulaError> {
        let support = self
            .support
            .get_mut(node_type)
            .ok_or(format!("node {node_type:?} was missing"))?;
        support.config_converter = Some(config_converter);
        Ok(())
    }

    pub fn add_default<N: DefaultFactoryRequirements>(&mut self) {
        self.add_factory(N::static_type(), Box::new(DefaultFactory::<N>::new()))
    }

    pub fn add_default_with_config<N: DefaultFactoryRequirements, C: DefaultConfigRequirements>(
        &mut self,
    ) -> Result<(), BetulaError> {
        self.add_factory(N::static_type(), Box::new(DefaultFactory::<N>::new()));
        self.add_config_converter(
            &N::static_type(),
            Box::new(DefaultConfigConverter::<C>::new()),
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_loader() -> Result<(), BetulaError> {
        let mut loader = TreeLoader::new();
        loader.add_default::<betula_core::nodes::SequenceNode>();
        loader.add_default::<betula_core::nodes::SelectorNode>();
        loader
            .add_default_with_config::<crate::nodes::DelayNode, crate::nodes::DelayNodeConfig>()?;
        println!("loader: {loader:#?}");
        Ok(())
    }
}
