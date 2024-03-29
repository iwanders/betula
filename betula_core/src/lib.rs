/*
    All behaviour tree execution is single threaded.

    Classical:
        Control flow (internal nodes):
            fallback, sequence, parallel, decorator
        Execution (leafs):
            action, condition


    Thoughts:
        Would be nice if the node ids were stable...


    The nodes can have state, the tree should use interior mutability.
    This is fine, as the callstack descends down the tree it should never
    encounter the same node twice, as that makes a loop and that sounds
    bad.

    Can we do something lazy? Where we only re-evaluate the parts of the
    tree that may have changed?

        We may be able to do something like that if we consider time to be a
        blackboard value?
        If nodes are pure functions, their return can't change if the inputs
        are the same, if the inputs (blackboard & children) are identical,
        we don't need to re-evaluate the parts. Which also means that
        we only have to evaluate from the from the first ancestor for which
        an input changed down. And we can stop executing if at any point we
        reach the prior state.
        If multiple trees share the same blackboard, we can always add a
        tick value on the blackboard, nodes that want to execute each tick
        can use the ticks as an input, and be guaranteed execution.
*/

pub mod basic;
pub mod blackboard;
pub mod nodes;

pub mod prelude {
    pub use crate::{
        as_any::AsAnyHelper, blackboard::Setup, AsAny, NodeConfigLoad, RunContext, Tree,
    };
}
pub use blackboard::BlackboardInterface;

mod as_any;
pub use as_any::AsAny;

use serde::{Deserialize, Serialize};

/// The result states returned by a node.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum NodeStatus {
    Running,
    Failure,
    Success,
}

/// The purest interface of a tree, used by the nodes to run their
/// children. The nodes don't have access to children directly.
pub trait RunContext {
    /// Get the number of immediate children.
    fn children(&self) -> usize;

    /// Run a child node.
    fn run(&self, index: usize) -> Result<NodeStatus, NodeError>;
}

/// The error type.
pub type BetulaError = Box<dyn std::error::Error + Send + Sync>;

/// Error type for results from node execution.
pub type NodeError = BetulaError;

// Output and Input feels ambiguous, is that from the blackboard or from
// the nodes?
/// Provider trait for nodes that set values.
pub trait ProviderTrait: std::fmt::Debug {
    type ProviderItem;
    fn set(&self, v: Self::ProviderItem) -> Result<(), NodeError>;
}

/// Consumer trait for nodes that get values.
pub trait ConsumerTrait: std::fmt::Debug {
    type ConsumerItem;
    fn get(&self) -> Result<Self::ConsumerItem, NodeError>;
}

#[derive(Debug)]
struct DefaultProviderConsumer<T> {
    z: std::marker::PhantomData<T>,
}
impl<T: std::fmt::Debug + 'static> ProviderTrait for DefaultProviderConsumer<T> {
    type ProviderItem = T;
    fn set(&self, _v: Self::ProviderItem) -> Result<(), NodeError> {
        Err("provider is not initialised".into())
    }
}
impl<T: std::fmt::Debug + 'static> ConsumerTrait for DefaultProviderConsumer<T> {
    type ConsumerItem = T;
    fn get(&self) -> Result<Self::ConsumerItem, NodeError> {
        Err("consumer is not initialised".into())
    }
}

/// Bidirectional trait for node sthat both read and write values.
pub trait ProviderConsumerTrait: ProviderTrait + ConsumerTrait + std::fmt::Debug {}
impl<T: std::fmt::Debug + 'static> ProviderConsumerTrait for DefaultProviderConsumer<T> {}

/// The boxed trait that nodes should use to provide values to the blackboard.
pub type Provider<T> = Box<dyn ProviderTrait<ProviderItem = T>>;
impl<T: std::fmt::Debug + 'static> Default for Provider<T> {
    fn default() -> Self {
        Box::new(DefaultProviderConsumer::<T> {
            z: std::marker::PhantomData,
        })
    }
}

/// The boxed trait that nodes should use to consume values from the blackboard.
pub type Consumer<T> = Box<dyn ConsumerTrait<ConsumerItem = T>>;
impl<T: std::fmt::Debug + 'static> Default for Consumer<T> {
    fn default() -> Self {
        Box::new(DefaultProviderConsumer::<T> {
            z: std::marker::PhantomData,
        })
    }
}

/// The boxed trait that nodes should use to provide and consume values from the blackboard.
pub type ProviderConsumer<T> = Box<dyn ProviderConsumerTrait<ProviderItem = T, ConsumerItem = T>>;
impl<T: std::fmt::Debug + 'static> Default for ProviderConsumer<T> {
    fn default() -> Self {
        Box::new(DefaultProviderConsumer::<T> {
            z: std::marker::PhantomData,
        })
    }
}

/// The type going across the port.
#[derive(Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct PortType {
    id: std::any::TypeId,
    type_name: &'static str,
}
impl PortType {
    pub fn new<T: 'static>() -> Self {
        PortType {
            id: std::any::TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
        }
    }
}

impl std::fmt::Debug for PortType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{0}", self.type_name)
    }
}

/// A port on a node with a name and type.
#[derive(Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct Port {
    /// The type of the data.
    pub data: PortType,
    /// The name of the port.
    pub name: String,
}

impl Port {
    pub fn new<T: 'static>(name: &str) -> Self {
        Port {
            data: PortType::new::<T>(),
            name: name.to_string(),
        }
    }
}

/// A port with a directionality.
pub enum DirectionalPort {
    /// The port consumes the value.
    Consumer(Port),
    /// The port provides the value.
    Provider(Port),
    /// The port provides and consumes the value.
    ProviderConsumer(Port),
}
impl DirectionalPort {
    pub fn consumer<T: 'static>(name: &str) -> Self {
        DirectionalPort::Consumer(Port::new::<T>(name))
    }
    pub fn provider<T: 'static>(name: &str) -> Self {
        DirectionalPort::Provider(Port::new::<T>(name))
    }
    pub fn provider_consumer<T: 'static>(name: &str) -> Self {
        DirectionalPort::ProviderConsumer(Port::new::<T>(name))
    }
    pub fn name(&self) -> &str {
        match self {
            DirectionalPort::Consumer(v) => v.name.as_ref(),
            DirectionalPort::Provider(v) => v.name.as_ref(),
            DirectionalPort::ProviderConsumer(v) => v.name.as_ref(),
        }
    }
}

/// Trait the configuration types must implement.
pub trait NodeConfig: std::fmt::Debug + AsAny {}
impl<T: std::fmt::Debug + 'static> NodeConfig for T {}

/// Helper trait to easily load types implementing clone, used in [`Node::set_config`]:
/// ```ignore
/// fn set_config(&mut self, config:  &dyn NodeConfig) -> Result<(), NodeError> {
///    self.config.load_node_config(config)
/// }
/// ```
pub trait NodeConfigLoad: NodeConfig {
    fn load_node_config(&mut self, v: &dyn NodeConfig) -> Result<(), BetulaError>
    where
        Self: Sized + 'static + Clone,
    {
        use crate::as_any::AsAnyHelper;
        let v = (*v).downcast_ref::<Self>().ok_or_else(|| {
            format!(
                "could not downcast {:?} to {:?}",
                (*v).type_name(),
                std::any::type_name::<Self>()
            )
        })?;
        *self = v.clone();
        Ok(())
    }
}
impl<T: NodeConfig> NodeConfigLoad for T {}
impl NodeConfigLoad for dyn NodeConfig + '_ {}

/// The type of a particular node.
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
pub struct NodeType(pub String);
impl From<&str> for NodeType {
    fn from(v: &str) -> Self {
        NodeType(v.to_owned())
    }
}
impl From<String> for NodeType {
    fn from(v: String) -> Self {
        NodeType(v.clone())
    }
}
impl Into<String> for NodeType {
    fn into(self) -> std::string::String {
        self.0.clone()
    }
}

/// Trait that nodes must implement.
pub trait Node: std::fmt::Debug + AsAny {
    /// The tick function for each node to perform actions / return status.
    ///   The return of Result is only there to indicate failure that would halt
    ///   behaviour tree execution on the spot. `Status::Failure` should be propagated
    ///   in the Ok() type.
    ///
    ///
    ///   self_id: The id of the current node being executed.
    ///   tree: The context in which this node is being ran.
    fn tick(&mut self, ctx: &dyn RunContext) -> Result<NodeStatus, NodeError>;

    /// Setup method for the node to obtain providers and consumers from the
    /// blackboard. Setup should happen mostly through the [`blackboard::Setup`] trait.
    /// The node should ONLY use the interface to register the specified port.
    fn port_setup(
        &mut self,
        port: &DirectionalPort,
        interface: &mut dyn BlackboardInterface,
    ) -> Result<(), NodeError> {
        let _ = (port, interface);
        Ok(())
    }

    /// Allow the node to express what ports it has.
    fn ports(&self) -> Result<Vec<DirectionalPort>, NodeError> {
        Ok(vec![])
    }

    /// Get a clone of the current configuration if any.
    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(None)
    }

    /// Set the config to the one provided.
    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        let _ = config;
        Ok(())
    }

    /// The human readable type of this node, must guarantee:
    /// ```ignore
    /// fn node_type(&self) -> NodeType {
    ///    Self::static_type()
    /// }
    /// ```
    fn node_type(&self) -> NodeType;

    /// Non object safe human readable type of this node.
    ///
    /// This is specified manually instead of [`std::any::type_name::<T>()`] because
    /// this allows for nodes to be moved around in refactors, as well as it being
    /// shorter than `betula_core::nodes::sequence_node::SequenceNode`.
    fn static_type() -> NodeType
    where
        Self: Sized;
}

use uuid::Uuid;

/// Node ids are represented as UUIDs.
///
/// We're using UUIDs as NodeIds here, that way we can guarantee that they
/// are stable, which helps a lot when manipulating the tree, internally
/// the tree is free to use whatever ids it wants when actually executing it.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
pub struct NodeId(pub Uuid);

/// Trait which a tree must implement.
///
/// All tree's are directed graphs. There may be multiple disjoint trees
/// present in one tree.
pub trait Tree {
    /// Return the ids present in this tree.
    fn nodes(&self) -> Vec<NodeId>;

    /// Return a reference to a node.
    fn node_ref(&self, id: NodeId) -> Option<&std::cell::RefCell<Box<dyn Node>>>;
    /// Return a mutable reference to a node.
    fn node_mut(&mut self, id: NodeId) -> Option<&mut dyn Node>;
    /// Removes a node and any relations associated to it.
    fn remove_node(&mut self, id: NodeId) -> Result<(), BetulaError>;
    /// Add a node to the tree.
    fn add_node_boxed(&mut self, id: NodeId, node: Box<dyn Node>) -> Result<NodeId, BetulaError>;

    /// Obtain a list of the children of a particular node.
    fn children(&self, id: NodeId) -> Result<Vec<NodeId>, BetulaError>;

    /// Add a relation between two nodes, specifying the insert position into the children
    /// vector.
    fn add_relation(
        &mut self,
        parent: NodeId,
        position: usize,
        child: NodeId,
    ) -> Result<(), BetulaError>;
    /// Remove a relation between two nodes, specifying the parent and the child position to remove.
    fn remove_relation(&mut self, parent: NodeId, position: usize) -> Result<(), BetulaError>;

    /// Execute the tick, starting at the provided node.
    fn execute(&self, id: NodeId) -> Result<NodeStatus, NodeError>;

    /// Call setup on a particular node.
    fn port_setup(
        &mut self,
        id: NodeId,
        port: &DirectionalPort,
        ctx: &mut dyn BlackboardInterface,
    ) -> Result<(), BetulaError>;
}
