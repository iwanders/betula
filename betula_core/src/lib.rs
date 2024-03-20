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
*/

pub mod basic;
pub mod blackboard;
pub mod nodes;

pub mod prelude {
    pub use crate::{blackboard::Setup, RunContext, Tree};
}
// Consumer, Error, Node, NodeId, Provider, Status

mod as_any;
pub use as_any::AsAny;

/// The result states returned by a node.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub enum Status {
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
    fn run(&self, index: usize) -> Result<Status, Error>;
}

/// The error type.
pub type Error = Box<dyn std::error::Error + Send + Sync>;

// Output and Input feels ambiguous, is that from the blackboard or from
// the nodes?
/// Provider trait for nodes that set values.
pub trait ProviderTrait: std::fmt::Debug {
    type ProviderItem;
    fn set(&self, v: Self::ProviderItem) -> Result<(), Error>;
}

/// Consumer trait for nodes that get values.
pub trait ConsumerTrait: std::fmt::Debug {
    type ConsumerItem;
    fn get(&self) -> Result<Self::ConsumerItem, Error>;
}

/// Bidirectional trait for node sthat both read and write values.
pub trait ProviderConsumerTrait: ProviderTrait + ConsumerTrait + std::fmt::Debug {}

/// The boxed trait that nodes should use to provide values to the blackboard.
pub type Provider<T> = Box<dyn ProviderTrait<ProviderItem = T>>;

/// The boxed trait that nodes should use to consume values from the blackboard.
pub type Consumer<T> = Box<dyn ConsumerTrait<ConsumerItem = T>>;

/// The boxed trait that nodes should use to provide and consume values from the blackboard.
pub type ProviderConsumer<T> = Box<dyn ProviderConsumerTrait<ProviderItem = T, ConsumerItem = T>>;

#[derive(Copy, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub struct Type {
    id: std::any::TypeId,
    type_name: &'static str,
}
impl Type {
    pub fn new<T: 'static>() -> Self {
        Type {
            id: std::any::TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
        }
    }
}

impl std::fmt::Debug for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{0}", self.type_name)
    }
}

#[derive(Clone, PartialEq, Eq, Ord, PartialOrd)]
pub struct Port {
    pub data: Type,
    pub name: String,
}

impl Port {
    pub fn new<T: 'static>(name: &str) -> Self {
        Port {
            data: Type::new::<T>(),
            name: name.to_string(),
        }
    }
}

pub enum DirectionalPort {
    Consumer(Port),
    Provider(Port),
    ProviderConsumer(Port),
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
    fn tick(&mut self, ctx: &dyn RunContext) -> Result<Status, Error>;

    /// Setup method for the node to obtain providers and consumers from the
    /// blackboard.
    fn setup(
        &mut self,
        _port: &DirectionalPort,
        _ctx: &mut dyn blackboard::Interface,
    ) -> Result<(), Error> {
        Ok(())
    }

    /// Allow the node to express what ports it has.
    fn ports(&self) -> Result<Vec<DirectionalPort>, Error> {
        Ok(vec![])
    }
}

use uuid::Uuid;

/// We're using UUIDs as NodeIds here, that way we can guarantee that they
/// are stable, which helps a lot when manipulating the tree, internally
/// the tree is free to use whatever ids it wants when actually executing it.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
pub struct NodeId(pub Uuid);

/// Trait which a tree must implement.
///
/// All tree's are directed graphs. There may be multiple disjoint trees
/// present in one tree.
pub trait Tree {
    /// Return the ids present in this tree.
    fn ids(&self) -> Vec<NodeId>;

    /// Return a reference to a node.
    // fn node_ref(&self, id: NodeId) -> Option<&dyn Node>;
    /// Return a mutable reference to a node.
    fn node_mut(&mut self, id: NodeId) -> Option<&mut dyn Node>;
    /// Removes a node and any relations associated to it.
    fn remove_node(&mut self, id: NodeId) -> Result<(), Error>;
    /// Add a node to the tree.
    fn add_node_boxed(&mut self, id: NodeId, node: Box<dyn Node>) -> Result<NodeId, Error>;

    /// Obtain a list of the children of a particular node.
    fn children(&self, id: NodeId) -> Result<Vec<NodeId>, Error>;

    /// Add a relation between two nodes, specifying the insert position into the children
    /// vector.
    fn add_relation(&mut self, parent: NodeId, position: usize, child: NodeId)
        -> Result<(), Error>;
    /// Remove a relation between two nodes, specifying the parent and the child position to remove.
    fn remove_relation(&mut self, parent: NodeId, position: usize) -> Result<(), Error>;

    /// Execute the tick, starting at the provided node.
    fn execute(&self, id: NodeId) -> Result<Status, Error>;

    /// Call setup on a particular node.
    fn setup(
        &mut self,
        id: NodeId,
        port: &DirectionalPort,
        ctx: &mut dyn blackboard::Interface,
    ) -> Result<(), Error>;
}
