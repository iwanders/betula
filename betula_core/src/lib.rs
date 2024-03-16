/*
    All behaviour tree execution is single threaded.
    Classical:
    Control flow:
        fallback, sequence, parallel, decorator
    Execution:
        action, condition

*/

mod nodes;
mod runner;

pub mod prelude {
    pub use crate::{Context, Error, Node, NodeId, Status, Tree};
}

/// Node Id that's used to refer to nodes in a context.
pub struct NodeId(usize);

/// The result states returned by a node.
pub enum Status {
    Running,
    Failure,
    Success,
}

/// The execution context passed to each tick.
pub trait Tree {
    /// Return a list of all node ids.
    fn nodes(&self) -> Vec<NodeId>;

    fn node(&self, id: NodeId) -> &dyn Node;
    fn node_mut(&mut self, id: NodeId) -> &mut dyn Node;

    /// Get the children of a particular node.
    fn children(&self, id: NodeId) -> Vec<NodeId>;

    /// Run a particular node by id, this will in turn run other nodes.
    fn run(&self, id: NodeId) -> Result<Status, Error>;
}

/// Do we need this?
pub trait Context {}

/// The error type.
pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub trait Node {
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
        ctx: &dyn Context,
    ) -> Result<Status, Error>;
}
