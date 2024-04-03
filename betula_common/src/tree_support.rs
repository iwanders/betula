use betula_core::prelude::*;
use betula_core::{BetulaError, Blackboard, Node, NodeConfig, NodeType};
use serde::{Deserialize, Serialize};

use crate::type_support::{
    // Config support.
    ConfigConverter,
    DefaultConfigConverter,
    DefaultConfigRequirements,
    // Node factories
    DefaultNodeFactory,
    DefaultNodeFactoryRequirements,
    // Blackboard value.
    DefaultValueConverter,
    DefaultValueRequirements,
    NodeFactory,
    ValueConverter,
};

pub type BlackboardFactory = Box<dyn Fn() -> Box<dyn Blackboard>>;

type SerializableHolder = serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SerializedConfig {
    node_type: NodeType,
    data: SerializableHolder,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SerializedValue {
    type_id: String,
    data: SerializableHolder,
}

mod v1 {
    use super::SerializableHolder;
    use betula_core::{BlackboardId, NodeId, PortConnection, PortName};
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct TreeNode {
        pub id: NodeId,
        pub node_type: String,
        pub config: Option<SerializableHolder>,
        pub children: Vec<NodeId>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct TypedValue {
        pub type_id: String,
        pub data: SerializableHolder,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Blackboard {
        pub id: BlackboardId,
        pub values: BTreeMap<PortName, TypedValue>,
        pub connections: Vec<PortConnection>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Root {
        pub nodes: Vec<TreeNode>,
        pub blackboards: Vec<Blackboard>,
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum Config {
    V1(v1::Root),
}

use std::collections::HashMap;
#[derive(Debug)]
struct NodeTypeSupport {
    factory: Box<dyn NodeFactory>,
    config_converter: Option<Box<dyn ConfigConverter>>,
}

#[derive(Debug)]
struct ValueTypeSupport {
    name: String,
    value_converter: Box<dyn ValueConverter>,
}

#[derive(Default)]
pub struct TreeSupport {
    node_support: HashMap<NodeType, NodeTypeSupport>,
    // technically, value_support should index based on the name.
    // but we more often serialize than deserialize, so lets keep this
    // as is for now.
    value_support: HashMap<std::any::TypeId, ValueTypeSupport>,
    blackboard_factory: Option<BlackboardFactory>,
}

impl std::fmt::Debug for TreeSupport {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let factory_string = self.blackboard_factory.as_ref().map(|_| "Factory");
        fmt.debug_struct("TreeSupport")
            .field("node_support", &self.node_support)
            .field("value_support", &self.value_support)
            .field("blackboard_factory", &factory_string)
            .finish()
    }
}

impl TreeSupport {
    pub fn new() -> Self {
        TreeSupport::default()
    }
    pub fn with_blackboard_factory(blackboard_factory: BlackboardFactory) -> Self {
        TreeSupport {
            blackboard_factory: Some(blackboard_factory),
            ..TreeSupport::default()
        }
    }

    pub fn set_blackboard_factory(&mut self, blackboard_factory: BlackboardFactory) {
        self.blackboard_factory = Some(blackboard_factory);
    }

    fn create_blackboard(&self) -> Option<Box<dyn Blackboard>> {
        self.blackboard_factory.as_ref().map(|v| v())
    }

    //fn get_node_types(&self) -> Vec<NodeType> {
    //    self.node_support.keys().cloned().collect()
    //}

    fn get_node_support(&self, node_type: &NodeType) -> Result<&NodeTypeSupport, BetulaError> {
        self.node_support
            .get(node_type)
            .ok_or(format!("could not get support for {node_type:?}").into())
    }

    pub fn create_node(&self, node_type: &NodeType) -> Result<Box<dyn Node>, BetulaError> {
        let node_support = self.get_node_support(node_type)?;

        node_support.factory.create()
    }

    fn get_value_type_support(&self, name: &str) -> Option<&ValueTypeSupport> {
        for (_, v) in self.value_support.iter() {
            if v.name == name {
                return Some(v);
            }
        }
        None
    }

    fn add_node_factory(&mut self, node_type: NodeType, factory: Box<dyn NodeFactory>) {
        let entry = self.node_support.insert(
            node_type.clone(),
            NodeTypeSupport {
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
            .node_support
            .get_mut(node_type)
            .ok_or(format!("node {node_type:?} was missing"))?;
        support.config_converter = Some(config_converter);
        Ok(())
    }

    pub fn add_node_default<N: DefaultNodeFactoryRequirements>(&mut self) {
        self.add_node_factory(N::static_type(), Box::new(DefaultNodeFactory::<N>::new()))
    }

    pub fn add_node_default_with_config<
        N: DefaultNodeFactoryRequirements,
        C: DefaultConfigRequirements,
    >(
        &mut self,
    ) {
        self.add_node_factory(N::static_type(), Box::new(DefaultNodeFactory::<N>::new()));
        self.add_config_converter(
            &N::static_type(),
            Box::new(DefaultConfigConverter::<C>::new()),
        )
        .expect("cannot fail, key was added line above");
    }

    pub fn add_value_default<V: DefaultValueRequirements>(&mut self) {
        let support = ValueTypeSupport {
            name: std::any::type_name::<V>().to_owned(),
            value_converter: Box::new(DefaultValueConverter::<V>::new()),
        };
        self.value_support
            .insert(std::any::TypeId::of::<V>(), support);
    }

    pub fn tree_serialize<S: serde::Serializer>(
        &self,
        tree: &dyn Tree,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut nodes = vec![];
        use betula_core::PortName;
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
            let config: Option<SerializableHolder> = if let Some(config) = config {
                let converter = self.node_support.get(&node_type);
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

        let mut blackboards = vec![];
        for id in tree.blackboards() {
            let connections = tree.blackboard_connections(id);
            let blackboard = tree
                .blackboard_ref(id)
                .ok_or(Error::custom(format!("could not get {id:?}")))?;
            let blackboard = blackboard.borrow();

            // Collect the values.
            let mut values: std::collections::BTreeMap<PortName, TypedValue> = Default::default();
            for port in blackboard.ports() {
                let value = blackboard.get(&port).ok_or(S::Error::custom(format!(
                    "could not get value for {port:?}"
                )))?;

                let value_type = (*value).as_any_type_id();

                let converter =
                    self.value_support
                        .get(&value_type)
                        .ok_or(S::Error::custom(format!(
                            "could not get converter for {:?}",
                            (*value).as_any_type_name()
                        )))?;

                let serialize_erased = converter
                    .value_converter
                    .value_serialize(&*value)
                    .map_err(|e| S::Error::custom(format!("failed with {e}")))?;
                let t = TypedValue {
                    type_id: converter.name.clone(),
                    data: serde_json::to_value(serialize_erased)
                        .map_err(|e| S::Error::custom(format!("json serialize error {e:?}")))?,
                };
                values.insert(port, t);
            }

            let b = Blackboard {
                id,
                values,
                connections,
            };
            blackboards.push(b);
        }

        // Make the results stable.
        blackboards.sort_by(|a, b| a.id.partial_cmp(&b.id).unwrap());
        for bb in blackboards.iter_mut() {
            bb.connections.sort();
        }
        nodes.sort_by(|a, b| a.id.partial_cmp(&b.id).unwrap());
        let root = Root { nodes, blackboards };
        let config = Config::V1(root);
        Ok(config
            .serialize(serializer)
            .map_err(|e| S::Error::custom(format!("serialize failed with {e:?}")))?)
    }

    pub fn tree_deserialize<'de, D: serde::Deserializer<'de>>(
        &self,
        tree: &mut dyn Tree,
        deserializer: D,
    ) -> Result<(), D::Error> {
        use betula_core::{blackboard::Chalkable, BlackboardId, PortConnection, PortName};
        use serde::de::Error;
        let config: Config = Config::deserialize(deserializer)?;

        match config {
            Config::V1(root) => {
                let mut relations = vec![];
                let mut new_nodes = vec![];

                // First, deserialize everything.
                for node in root.nodes {
                    let node_type = node.node_type.into();

                    let mut new_node = self
                        .create_node(&node_type)
                        .map_err(|e| D::Error::custom(format!("failed to construct node {e:?}")))?;

                    if let Some(config) = node.config {
                        let node_support = self.get_node_support(&node_type).map_err(|e| {
                            D::Error::custom(format!(
                                "failed to get node support for {node_type:?}: {e:?}"
                            ))
                        })?;
                        if let Some(config_support) = node_support.config_converter.as_ref() {
                            let mut erased =
                                Box::new(<dyn erased_serde::Deserializer>::erase(config));
                            let new_config = config_support
                                .config_deserialize(&mut erased)
                                .map_err(|e| {
                                    D::Error::custom(format!("failed deserialize config {e:?}"))
                                })?;
                            println!("new_config: {new_config:?}");
                            new_node.set_config(&*new_config).map_err(|e| {
                                D::Error::custom(format!("failed set config {e:?}"))
                            })?;
                        }
                    }
                    new_nodes.push((node.id, new_node));
                    relations.push((node.id, node.children));
                }
                // deserialize the blackboards.
                struct BlackboardDeserialized {
                    pub id: BlackboardId,
                    pub values: HashMap<PortName, Box<dyn Chalkable>>,
                    pub connections: Vec<PortConnection>,
                }
                let mut blackboards: Vec<BlackboardDeserialized> = vec![];
                for blackboard in root.blackboards {
                    let mut deserialized_bb = BlackboardDeserialized {
                        id: blackboard.id,
                        connections: blackboard.connections.clone(),
                        values: Default::default(),
                    };
                    for (k, v) in blackboard.values {
                        let v1::TypedValue { type_id, data } = v;
                        let support =
                            self.get_value_type_support(&type_id)
                                .ok_or(D::Error::custom(format!(
                                    "could not get value support for {type_id:?}"
                                )))?;
                        // Now, convert it to the boxed value.
                        let mut erased = Box::new(<dyn erased_serde::Deserializer>::erase(data));
                        let boxed_value = support
                            .value_converter
                            .value_deserialize(&mut erased)
                            .map_err(|e| {
                                D::Error::custom(format!("failed deserialize value {e:?}"))
                            })?;
                        deserialized_bb.values.insert(k.clone(), boxed_value);
                    }
                    blackboards.push(deserialized_bb);
                }

                // Serialization is all done, now add the nodes to the tree.
                for (node_id, node) in new_nodes {
                    tree.add_node_boxed(node_id, node)
                        .map_err(|e| D::Error::custom(format!("failed to add new node {e:?}")))?;
                }

                // Create the connections.
                for (parent, children) in relations {
                    tree.set_children(parent, &children)
                        .map_err(|e| D::Error::custom(format!("failed to relation {e:?}")))?;
                }

                // Add the blackboards
                for blackboard in blackboards {
                    let id = blackboard.id;
                    let mut bb = self
                        .create_blackboard()
                        .ok_or(D::Error::custom(format!("no blackboard factory function")))?;
                    for (k, v) in blackboard.values {
                        bb.set(&k, v.clone()).map_err(|e| {
                            D::Error::custom(format!(
                                "failed to set {k:?} to {v:?} on {id:?}: {e:?}"
                            ))
                        })?;
                    }
                    tree.add_blackboard_boxed(id, bb).map_err(|e| {
                        D::Error::custom(format!("failed to add blackboard with {id:?}: {e:?}"))
                    })?;
                    for connection in blackboard.connections {
                        tree.connect_port(&connection).map_err(|e| D::Error::custom(format!("failed to make connection {connection:?} for blackboard {id:?}: {e:?}")))?;
                    }
                }
            }
        }

        Ok(())
    }

    pub fn config_serialize(
        &self,
        node_type: NodeType,
        config: &dyn NodeConfig,
    ) -> Result<SerializedConfig, BetulaError> {
        let converter = self.node_support.get(&node_type);
        let converter = converter
            .map(|v| v.config_converter.as_ref())
            .ok_or(format!(
                "config_serialize: could not get support for {node_type:?}"
            ))?
            .ok_or(format!(
                "config_serialize: could not get config serializer for {node_type:?}"
            ))?;
        let serialize_erased = converter.config_serialize(&*config)?;
        Ok(SerializedConfig {
            node_type: node_type,
            data: serde_json::to_value(serialize_erased)
                .map_err(|e| format!("json serialize error {e:?}"))?,
        })
    }

    pub fn config_deserialize(
        &self,
        config: SerializedConfig,
    ) -> Result<Box<dyn NodeConfig>, BetulaError> {
        let node_type = &config.node_type;
        let converter = self.node_support.get(&node_type);
        let converter = converter
            .map(|v| v.config_converter.as_ref())
            .ok_or(format!(
                "config_deserialize: could not get support for {node_type:?}"
            ))?
            .ok_or(format!(
                "config_deserialize: could not get config serializer for {node_type:?}"
            ))?;
        let mut erased = Box::new(<dyn erased_serde::Deserializer>::erase(config.data));
        Ok(converter.config_deserialize(&mut erased)?)
    }
}

pub struct TreeSerializer<'a, 'b> {
    config_support: &'b TreeSupport,
    tree: &'a dyn Tree,
}

impl<'a, 'b> TreeSerializer<'a, 'b> {
    pub fn new(config_support: &'b TreeSupport, tree: &'a dyn Tree) -> Self {
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
        self.config_support.tree_serialize(self.tree, serializer)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use betula_core::basic::{BasicBlackboard, BasicTree};
    use betula_core::nodes::{FailureNode, SelectorNode, SuccessNode};
    use betula_core::{BlackboardId, NodeId};
    use uuid::Uuid;
    #[test]
    fn test_config() -> Result<(), BetulaError> {
        let mut tree_support = TreeSupport::new();
        use crate::nodes::{DelayNode, DelayNodeConfig};
        tree_support.add_node_default_with_config::<DelayNode, DelayNodeConfig>();
        let interval = 3.3;
        let config: Box<dyn NodeConfig> = Box::new(DelayNodeConfig { interval });
        let serialized = tree_support.config_serialize(DelayNode::static_type(), &*config)?;
        let deserialized_box = tree_support.config_deserialize(serialized)?;
        let deserialized = (*deserialized_box)
            .downcast_ref::<DelayNodeConfig>()
            .ok_or(format!("could not downcast"))?;
        assert_eq!(interval, deserialized.interval);
        Ok(())
    }
    #[test]
    fn test_tree() -> Result<(), BetulaError> {
        let mut tree_support = TreeSupport::new();
        tree_support.add_node_default::<betula_core::nodes::SequenceNode>();
        tree_support.add_node_default::<betula_core::nodes::SelectorNode>();
        tree_support.add_node_default::<betula_core::nodes::FailureNode>();
        tree_support.add_node_default::<betula_core::nodes::SuccessNode>();
        tree_support
            .add_node_default_with_config::<crate::nodes::DelayNode, crate::nodes::DelayNodeConfig>(
            );
        println!("loader: {tree_support:#?}");

        // Lets make a new tree.
        let mut tree: Box<dyn Tree> = Box::new(BasicTree::new());
        let root = tree.add_node_boxed(NodeId(Uuid::new_v4()), Box::new(SelectorNode {}))?;
        let f1 = tree.add_node_boxed(NodeId(Uuid::new_v4()), Box::new(FailureNode {}))?;
        let s1 = tree.add_node_boxed(NodeId(Uuid::new_v4()), Box::new(SuccessNode {}))?;
        tree.set_children(root, &vec![f1, s1])?;

        let obj = TreeSerializer::new(&tree_support, &*tree);
        let config_json = serde_json::to_string(&obj)?;
        println!("config json: {config_json:?}");

        let json_value = tree_support.tree_serialize(&*tree, serde_json::value::Serializer)?;
        println!("json_value: {json_value}");

        // lets try to rebuild the tree from that json value.
        let mut new_tree: Box<dyn Tree> = Box::new(BasicTree::new());
        tree_support.tree_deserialize(&mut *new_tree, json_value.clone())?;
        println!("new_tree: {new_tree:#?}");
        let and_back = tree_support.tree_serialize(&*new_tree, serde_json::value::Serializer)?;
        assert_eq!(and_back, json_value);

        // let mut another_tree = tree_support.deserialize_default::<BasicTree,_>(json_value.clone())?;
        // let and_another_back = tree_support.serialize(&tree, serde_json::value::Serializer)?;
        // assert_eq!(and_another_back, json_value);

        Ok(())
    }

    #[test]
    fn test_with_blackboard() -> Result<(), BetulaError> {
        let mut tree_support = TreeSupport::new();
        tree_support.add_node_default::<betula_core::nodes::SequenceNode>();
        tree_support.add_node_default::<betula_core::nodes::SelectorNode>();
        tree_support.add_node_default::<betula_core::nodes::FailureNode>();
        tree_support.add_node_default::<betula_core::nodes::SuccessNode>();
        tree_support
            .add_node_default_with_config::<crate::nodes::DelayNode, crate::nodes::DelayNodeConfig>(
            );
        tree_support.add_node_default::<crate::nodes::TimeNode>();
        tree_support.add_value_default::<f64>();

        let mut tree: Box<dyn Tree> = Box::new(BasicTree::new());
        let root = tree.add_node_boxed(
            NodeId(Uuid::new_v4()),
            Box::new(betula_core::nodes::SequenceNode {}),
        )?;
        let time_node = tree.add_node_boxed(
            NodeId(Uuid::new_v4()),
            Box::new(crate::nodes::TimeNode::default()),
        )?;
        let delay_node = tree.add_node_boxed(
            NodeId(Uuid::new_v4()),
            Box::new(crate::nodes::DelayNode::default()),
        )?;
        tree.set_children(root, &vec![time_node, delay_node])?;

        // Add the blackboard.
        let bb = tree.add_blackboard_boxed(
            BlackboardId(Uuid::new_v4()),
            Box::new(BasicBlackboard::default()),
        )?;

        let output_ports = tree.node_ports(time_node)?;
        tree.connect_port_to_blackboard(&output_ports[0], bb)?;
        let input_ports = tree.node_ports(delay_node)?;
        tree.connect_port_to_blackboard(&input_ports[0], bb)?;

        let obj = TreeSerializer::new(&tree_support, &*tree);
        let config_json = serde_json::to_string(&obj)?;
        println!("config_json: {config_json:?}");

        let json_value = tree_support.tree_serialize(&*tree, serde_json::value::Serializer)?;
        println!("json_value: {json_value:#?}");

        tree_support.set_blackboard_factory(Box::new(|| Box::new(BasicBlackboard::default())));

        // lets try to rebuild the tree from that json value.
        let mut new_tree: Box<dyn Tree> = Box::new(BasicTree::new());
        tree_support.tree_deserialize(&mut *new_tree, json_value.clone())?;
        println!("new_tree: {new_tree:#?}");
        let and_back = tree_support.tree_serialize(&*new_tree, serde_json::value::Serializer)?;
        assert_eq!(and_back, json_value);

        Ok(())
    }
}
