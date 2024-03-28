use betula_core::prelude::*;
use betula_core::{BetulaError, NodeType};
use serde::{Deserialize, Serialize};

use crate::support::{
    ConfigConverter, DefaultConfigConverter, DefaultConfigRequirements, DefaultFactory,
    DefaultFactoryRequirements, NodeFactory,
};

mod v1 {
    use betula_core::NodeId;
    use serde::{Deserialize, Serialize};
    pub type SerializableNodeConfig = serde_json::Value;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct TreeNode {
        pub id: NodeId,
        pub node_type: String,
        pub config: Option<SerializableNodeConfig>,
        pub children: Vec<NodeId>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct TreeNodes {
        pub nodes: Vec<TreeNode>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Root {
        pub tree: TreeNodes,
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum Config {
    V1(v1::Root),
}

use std::collections::HashMap;
#[derive(Debug)]
struct TypeSupport {
    factory: Box<dyn NodeFactory>,
    config_converter: Option<Box<dyn ConfigConverter>>,
}
#[derive(Debug, Default)]
pub struct TreeConfig {
    support: HashMap<NodeType, TypeSupport>,
}
impl TreeConfig {
    pub fn new() -> Self {
        TreeConfig::default()
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

    pub fn serialize<S: serde::Serializer>(
        &self,
        tree: &dyn Tree,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut nodes = vec![];
        use serde::ser::Error;
        use v1::*;
        for id in tree.nodes() {
            let tree_node = tree
                .node_ref(id)
                .ok_or(Error::custom(format!("could not get {id:?}")))?;
            let tree_node = tree_node.borrow();
            let config = tree_node
                .get_config()
                .map_err(|e| S::Error::custom(format!("could not get config {e:?}")))?;
            let node_type = tree_node.node_type();
            let config: Option<SerializableNodeConfig> = if let Some(config) = config {
                let converter = self.support.get(&node_type);
                let converter = converter
                    .map(|v| v.config_converter.as_ref())
                    .flatten()
                    .ok_or(S::Error::custom(format!(
                        "could not get support for {node_type:?}"
                    )))?;
                let serialize_erased = converter
                    .config_serialize(&*config)
                    .map_err(|e| S::Error::custom(format!("failed with {e}")))?;
                Some(
                    serde_json::to_value(serialize_erased)
                        .map_err(|e| S::Error::custom(format!("json serialize error {e:?}")))?,
                )
            } else {
                None
            };

            let children = tree
                .children(id)
                .map_err(|e| S::Error::custom(format!("could not get children {e:?}")))?;
            let this_node = TreeNode {
                id,
                node_type: node_type.into(),
                config,
                children,
            };
            nodes.push(this_node);
        }

        let root = Root {
            tree: TreeNodes { nodes },
        };
        let config = Config::V1(root);
        Ok(config
            .serialize(serializer)
            .map_err(|e| S::Error::custom(format!("serialize failed with {e:?}")))?)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        &self,
        tree: &mut dyn Tree,
        deserializer: D,
    ) -> Result<(), D::Error> {
        use serde::de::Error;
        let config: Config = Config::deserialize(deserializer)?;

        // pub id: NodeId,
        // pub node_type: String,
        // pub config: Option<SerializableNodeConfig>,
        // pub children: Vec<NodeId>,
        match config {
            Config::V1(root) => {
                // let mut relations = vec![];
                for node in root.tree.nodes {
                    let support = self.support.get(&node.node_type);
                }
            }
        }

        Ok(())
    }
}

struct TreeSerializer<'a, 'b> {
    tree: &'a dyn Tree,
    config_support: &'b TreeConfig,
}

impl<'a, 'b> serde::Serialize for TreeSerializer<'a, 'b> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.config_support.serialize(self.tree, serializer)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use betula_core::basic::BasicTree;
    use betula_core::nodes::{FailureNode, SelectorNode, SuccessNode};
    use betula_core::NodeId;
    use uuid::Uuid;
    #[test]
    fn test_config() -> Result<(), BetulaError> {
        let mut tree_config = TreeConfig::new();
        tree_config.add_default::<betula_core::nodes::SequenceNode>();
        tree_config.add_default::<betula_core::nodes::SelectorNode>();
        tree_config
            .add_default_with_config::<crate::nodes::DelayNode, crate::nodes::DelayNodeConfig>()?;
        println!("loader: {tree_config:#?}");

        // Lets make a new tree.
        let mut tree = BasicTree::new();
        let root = tree.add_node_boxed(NodeId(Uuid::new_v4()), Box::new(SelectorNode {}))?;
        let f1 = tree.add_node_boxed(NodeId(Uuid::new_v4()), Box::new(FailureNode {}))?;
        let s1 = tree.add_node_boxed(NodeId(Uuid::new_v4()), Box::new(SuccessNode {}))?;
        tree.add_relation(root, 0, f1)?;
        tree.add_relation(root, 1, s1)?;

        let obj = TreeSerializer {
            tree: &tree,
            config_support: &tree_config,
        };
        let config_json = serde_json::to_string(&obj)?;
        println!("config json: {config_json:?}");

        let json_value = tree_config.serialize(&tree, serde_json::value::Serializer)?;
        println!("json_value: {json_value}");

        Ok(())
    }
}
