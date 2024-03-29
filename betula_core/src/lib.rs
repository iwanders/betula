/*!

The core traits and requirements for a Betula Behaviour Tree.

The [`Tree`] holds two fundamental types:
* [`Node`] these are the nodes making up the behaviour tree.
* [`Blackboard`] these are blackboards that nodes interface with for data.

All behaviour tree execution is single threaded.

The nodes can have state, the tree should use interior mutability.
This is fine, as the callstack descends down the tree it should never
encounter the same node twice, as that makes a loop, loops in a behaviour
tree don't really make sense.

On blackboards:
* Blackboards are key-value stores.
* Nodes may consume data, these are inputs.
* Nodes may provide data, these are outputs.
* Name remaps happen at the input side. Such that one output
  can still be uniquely referred to, but write to one blackboard under
  different names.
* Input and outputs cannot write to each other directly, they MUST pass
  through a blackboard. This allows the tree to track writes and decide if
  parts of the tree must be re-evaluated.

*/

/*
    Classical:
        Control flow (internal nodes):
            fallback, sequence, parallel, decorator
        Execution (leafs):
            action, condition


    Thoughts:
        Would be nice if the node ids were stable...


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
        as_any::AsAnyHelper, blackboard::Chalkable, blackboard::Setup, AsAny, NodeConfigLoad,
        RunContext, Tree,
    };
}
pub use blackboard::BlackboardInterface;

mod as_any;
pub use as_any::AsAny;

use uuid::Uuid;

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

/// Output trait for nodes that set values.
pub trait OutputTrait: std::fmt::Debug {
    type OutputItem;
    fn set(&self, v: Self::OutputItem) -> Result<(), NodeError>;
}

/// Input trait for nodes that get values.
pub trait InputTrait: std::fmt::Debug {
    type InputItem;
    fn get(&self) -> Result<Self::InputItem, NodeError>;
}

#[derive(Debug)]
struct DefaultOutputInput<T> {
    z: std::marker::PhantomData<T>,
}
impl<T: std::fmt::Debug + 'static> OutputTrait for DefaultOutputInput<T> {
    type OutputItem = T;
    fn set(&self, _v: Self::OutputItem) -> Result<(), NodeError> {
        Err("output is not initialised".into())
    }
}
impl<T: std::fmt::Debug + 'static> InputTrait for DefaultOutputInput<T> {
    type InputItem = T;
    fn get(&self) -> Result<Self::InputItem, NodeError> {
        Err("input is not initialised".into())
    }
}

/// The boxed trait that nodes should use to provide values to the blackboard.
pub type Output<T> = Box<dyn OutputTrait<OutputItem = T>>;
impl<T: std::fmt::Debug + 'static> Default for Output<T> {
    fn default() -> Self {
        Box::new(DefaultOutputInput::<T> {
            z: std::marker::PhantomData,
        })
    }
}

/// The boxed trait that nodes should use to consume values from the blackboard.
pub type Input<T> = Box<dyn InputTrait<InputItem = T>>;
impl<T: std::fmt::Debug + 'static> Default for Input<T> {
    fn default() -> Self {
        Box::new(DefaultOutputInput::<T> {
            z: std::marker::PhantomData,
        })
    }
}

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
pub struct PortName(pub String);
impl From<&str> for PortName {
    fn from(v: &str) -> Self {
        PortName(v.to_owned())
    }
}
impl std::ops::Deref for PortName {
    type Target = str;
    fn deref(&self) -> &<Self as std::ops::Deref>::Target {
        self.0.as_ref()
    }
}

impl From<String> for PortName {
    fn from(v: String) -> Self {
        PortName(v.clone())
    }
}
impl Into<String> for PortName {
    fn into(self) -> std::string::String {
        self.0.clone()
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

/// A port with a directionality.
///
/// Input ports on a node take inputs by this name. Output ports provide an
/// output by the specified name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum PortDirection {
    /// The port consumes the value, it is an input to the node.
    Input,
    /// The port provides the value, it is an output from the node.
    Output,
}

/// A port for a node.
///
/// Ports have a name, direction and type.
#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct Port {
    port_type: PortType,
    direction: PortDirection,
    name: PortName,
}

impl Port {
    pub fn input<T: 'static>(name: &PortName) -> Self {
        Port {
            port_type: PortType::new::<T>(),
            direction: PortDirection::Input,
            name: name.clone(),
        }
    }
    pub fn output<T: 'static>(name: &PortName) -> Self {
        Port {
            port_type: PortType::new::<T>(),
            direction: PortDirection::Output,
            name: name.clone(),
        }
    }

    pub fn into_node_port(self, node: NodeId) -> NodePort {
        NodePort {
            node,
            direction: self.direction,
            name: self.name,
        }
    }

    pub fn port_type(&self) -> PortType {
        self.port_type.clone()
    }
    pub fn direction(&self) -> PortDirection {
        self.direction.clone()
    }
    pub fn name(&self) -> PortName {
        self.name.clone()
    }
}
use crate::blackboard::Blackboard;

/// An untyped identifier for a node's port.
#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct NodePort {
    node: NodeId,
    direction: PortDirection,
    name: PortName,
}
impl NodePort {
    pub fn new(node: NodeId, name: &PortName, direction: PortDirection) -> Self {
        NodePort {
            node,
            direction,
            name: name.clone(),
        }
    }
    pub fn node(&self) -> NodeId {
        self.node.clone()
    }
    pub fn name(&self) -> PortName {
        self.name.clone()
    }
    pub fn direction(&self) -> PortDirection {
        self.direction.clone()
    }
}

/// An untyped identifier for a node's output port.
#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct BlackboardPort {
    blackboard: BlackboardId,
    name: PortName,
}
impl BlackboardPort {
    pub fn new(blackboard: BlackboardId, name: &PortName) -> Self {
        BlackboardPort {
            blackboard,
            name: name.clone(),
        }
    }
    pub fn blackboard(&self) -> BlackboardId {
        self.blackboard.clone()
    }
    pub fn name(&self) -> PortName {
        self.name.clone()
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

    /// Setup method for the node to obtain outputs and inputs from the
    /// blackboard. Setup should happen mostly through the [`blackboard::Setup`] trait.
    /// The node should ONLY use the interface to register the specified port.
    fn port_setup(
        &mut self,
        port: &PortName,
        direction: PortDirection,
        interface: &mut dyn BlackboardInterface,
    ) -> Result<(), NodeError> {
        let _ = (port, direction, interface);
        Ok(())
    }

    /// Allow the node to express what ports it has.
    fn ports(&self) -> Result<Vec<Port>, NodeError> {
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

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
pub struct BlackboardId(pub Uuid);

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
    fn remove_node(&mut self, id: NodeId) -> Result<Box<dyn Node>, BetulaError>;
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

    /// Get a list of the blackboard ids.
    fn blackboards(&self) -> Vec<BlackboardId>;

    /// Return a reference to a blackboard.
    fn blackboard_ref(&self, id: BlackboardId) -> Option<&std::cell::RefCell<Box<dyn Blackboard>>>;

    /// Return a mutable reference to a blackboard.
    fn blackboard_mut(&mut self, id: BlackboardId) -> Option<&mut dyn Blackboard>;

    /// Add a new blackboard to the tree.
    fn add_blackboard_boxed(
        &mut self,
        id: BlackboardId,
        blackboard: Box<dyn Blackboard>,
    ) -> Result<BlackboardId, BetulaError>;

    /// Remove a blackboard by the specified id.
    fn remove_blackboard(&mut self, id: BlackboardId) -> Option<Box<dyn Blackboard>>;

    /// Connect an input or an output port to a blackboard using the port's name.
    fn connect_port_to_blackboard(
        &mut self,
        node_port: &NodePort,
        blackboard: BlackboardId,
    ) -> Result<(), BetulaError> {
        self.connect_port_to_blackboard_port(
            node_port,
            &BlackboardPort::new(blackboard, &node_port.name()),
        )
    }

    /// Connect an input or an output to a blackboard, using the specified blackboard port name.
    fn connect_port_to_blackboard_port(
        &mut self,
        node_port: &NodePort,
        blackboard_port: &BlackboardPort,
    ) -> Result<(), BetulaError>;

    /// Disconnect a connection between a node's port and a blackboard's port.
    fn disconnect_port(
        &mut self,
        node_port: &NodePort,
        blackboard_port: &BlackboardPort,
    ) -> Result<(), BetulaError>;

    /// List all port connections.
    fn port_connections(&self) -> Vec<(NodePort, BlackboardPort)>;

    /// List all ports of a node
    fn node_ports(&self, node: NodeId) -> Result<Vec<NodePort>, NodeError> {
        let node_ref = self
            .node_ref(node)
            .ok_or_else(|| format!("could not find {node:?}"))?;
        let borrow = node_ref.try_borrow()?;
        let node_ports = borrow
            .ports()?
            .drain(..)
            .map(|z| z.clone().into_node_port(node))
            .collect::<Vec<_>>();
        Ok(node_ports)
    }
}
