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
    pub use crate::{
        blackboard::BlackboardSetup, Consumer, Error, Node, Provider, RunContext, Status,
    };
}

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

    // We probably want clone here, such that we can duplicate from the
    // ui.

    /// Setup method for the node to obtain providers and consumers from the
    /// blackboard.
    fn setup(&mut self, _ctx: &mut dyn blackboard::BlackboardInterface) -> Result<(), Error> {
        Ok(())
    }
}
