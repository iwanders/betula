/*
    All behaviour tree execution is single threaded.

    Classical:
        Control flow (internal nodes):
            fallback, sequence, parallel, decorator
        Execution (leafs):
            action, condition

*/

pub mod basic;
pub mod nodes;

pub mod prelude {
    pub use crate::{Context, Error, Node, NodeId, Status, Tree};
}

/// Node Id that's used to refer to nodes in a context.
#[derive(Copy, Clone, Debug)]
pub struct NodeId(pub usize);

/// The result states returned by a node.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub enum Status {
    Running,
    Failure,
    Success,
}

/// The execution context passed to each tick.
pub trait Tree {
    /// Return a list of all node ids.
    fn nodes(&self) -> Vec<NodeId>;

    /// Add a node to the tree, returning the NodeId for the node that was
    /// just added.
    fn add_node(&mut self, node: Box<dyn Node>) -> NodeId;

    /// Add a relation between a parent and a child.
    fn add_relation(&mut self, parent: NodeId, child: NodeId);

    /// Get the children of a particular node.
    fn children(&self, id: NodeId) -> Vec<NodeId>;

    /// Run a particular node by id, this will in turn run other nodes.
    /// Nodes do NOT have direct access to other nodes, instead they must
    /// call other nodes (including their children) through this method.
    fn run(&self, id: NodeId) -> Result<Status, Error>;
}

/// Do we need this, yes, but it needs some more thinking.
pub trait Context {}

/// The error type.
pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub trait Node: std::fmt::Debug {
    /// The tick function for each node to perform actions / return status.
    ///   The return of Result is only there to indicate failure that would halt
    ///   behaviour tree execution on the spot. `Status::Failure` should be propagated
    ///   in the Ok() type.
    ///
    ///
    ///   self_id: The id of the current node being executed.
    ///   tree: The context in which this node is being ran.
    fn tick(
        &mut self,
        self_id: NodeId,
        tree: &dyn Tree,
        ctx: &mut dyn Context,
    ) -> Result<Status, Error>;

    // We probably want clone here, such that we can duplicate from the
    // ui.
}
