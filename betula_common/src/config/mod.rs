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
            node_type.clone(),
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

        match config {
            Config::V1(root) => {
                let mut relations = vec![];
                let mut new_nodes = vec![];

                // First, deserialize everything.
                for node in root.tree.nodes {
                    let node_type = node.node_type.into();
                    let support = self
                        .support
                        .get(&node_type)
                        .ok_or(D::Error::custom(format!(
                            "could not get support for {node_type:?}"
                        )))?;
                    let mut new_node = support
                        .factory
                        .create()
                        .map_err(|e| D::Error::custom(format!("failed to construct node {e:?}")))?;
                    if let Some(config) = node.config {
                        if let Some(config_support) = support.config_converter.as_ref() {
                            let mut erased =
                                Box::new(<dyn erased_serde::Deserializer>::erase(config));
                            let new_config = config_support
                                .config_deserialize(&mut erased)
                                .map_err(|e| {
                                    D::Error::custom(format!("failed deserialize config {e:?}"))
                                })?;
                            new_node.set_config(&new_config).map_err(|e| {
                                D::Error::custom(format!("failed set config {e:?}"))
                            })?;
                        }
                    }
                    new_nodes.push((node.id, new_node));
                    relations.push((node.id, node.children));
                }
                // Serialization is all done, now add them to the tree.
                for (node_id, node) in new_nodes {
                    tree.add_node_boxed(node_id, node)
                        .map_err(|e| D::Error::custom(format!("failed to add new node {e:?}")))?;
                }
                for (parent, children) in relations {
                    for (i, child) in children.iter().enumerate() {
                        tree.add_relation(parent, i, *child)
                            .map_err(|e| D::Error::custom(format!("failed to relation {e:?}")))?;
                    }
                }
            }
        }

        Ok(())
    }

    // pub fn deserialize_default<'de, T: Tree + Default, D: serde::Deserializer<'de>, >(&self, deserializer: D) -> Result<T, D::Error> {
    // let mut tree = T::default();
    // self.deserialize(&mut tree, deserializer)?;
    // Ok(tree)
    // }
}

pub struct TreeSerializer<'a, 'b> {
    config_support: &'b TreeConfig,
    tree: &'a dyn Tree,
}

impl<'a, 'b> TreeSerializer<'a, 'b> {
    pub fn new(config_support: &'b TreeConfig, tree: &'a dyn Tree) -> Self {
        Self {
            tree,
            config_support,
        }
    }
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
        tree_config.add_default::<betula_core::nodes::FailureNode>();
        tree_config.add_default::<betula_core::nodes::SuccessNode>();
        tree_config
            .add_default_with_config::<crate::nodes::DelayNode, crate::nodes::DelayNodeConfig>()?;
        println!("loader: {tree_config:#?}");

        // Lets make a new tree.
        let mut tree: Box<dyn Tree> = Box::new(BasicTree::new());
        let root = tree.add_node_boxed(NodeId(Uuid::new_v4()), Box::new(SelectorNode {}))?;
        let f1 = tree.add_node_boxed(NodeId(Uuid::new_v4()), Box::new(FailureNode {}))?;
        let s1 = tree.add_node_boxed(NodeId(Uuid::new_v4()), Box::new(SuccessNode {}))?;
        tree.add_relation(root, 0, f1)?;
        tree.add_relation(root, 1, s1)?;

        let obj = TreeSerializer::new(&tree_config, &*tree);
        let config_json = serde_json::to_string(&obj)?;
        println!("config json: {config_json:?}");

        let json_value = tree_config.serialize(&*tree, serde_json::value::Serializer)?;
        println!("json_value: {json_value}");

        // lets try to rebuild the tree from that json value.
        let mut new_tree: Box<dyn Tree> = Box::new(BasicTree::new());
        tree_config.deserialize(&mut *new_tree, json_value.clone())?;
        println!("new_tree: {new_tree:#?}");
        let and_back = tree_config.serialize(&*tree, serde_json::value::Serializer)?;
        assert_eq!(and_back, json_value);

        // let mut another_tree = tree_config.deserialize_default::<BasicTree,_>(json_value.clone())?;
        // let and_another_back = tree_config.serialize(&tree, serde_json::value::Serializer)?;
        // assert_eq!(and_another_back, json_value);

        // Lets add a blackboard.

        Ok(())
    }
}
