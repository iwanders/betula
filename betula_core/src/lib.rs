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
pub mod nodes;

pub mod prelude {
    pub use crate::{Error, Node, RunContext, Status};
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
pub trait Provider {
    type ProviderItem;
    fn set(&self, v: Self::ProviderItem) -> Result<Self::ProviderItem, Error>;
}

/// Consumer trait for nodes that get values.
pub trait Consumer {
    type ConsumerItem;
    fn get(&self) -> Result<Self::ConsumerItem, Error>;
}

/// Bidirectional trait for node sthat both read and write values.
pub trait ProviderConsumer: Provider + Consumer {}

// We do this dance here because we can't have generic methods on an object
// safe trait, this way we still provide an abstraction over the actual
// setup type, but we can obtain concrete objects easily.
use std::any::TypeId;
pub type BlackboardValueCreator = Box<dyn FnOnce() -> Box<dyn std::any::Any>>;
pub trait BlackboardContext {
    fn provides(
        &mut self,
        id: &TypeId,
        key: &str,
        default: BlackboardValueCreator,
    ) -> Box<dyn std::any::Any>;
    fn consumes(&mut self, id: &TypeId, key: &str) -> Box<dyn std::any::Any>;
    // fn provides_consumes(&mut self, id: &TypeId, key: &str, default: BlackboardValueCreator) -> Box<dyn std::any::Any>;
}

pub type ProviderRc<T> = Rc<dyn Provider<ProviderItem = T>>;
pub type ConsumerRc<T> = Rc<dyn Consumer<ConsumerItem = T>>;
pub type ProviderConsumerRc<T> = Rc<dyn ProviderConsumer<ProviderItem = T, ConsumerItem = T>>;
use std::rc::Rc;
pub struct BlackboardWrapper<'a> {
    ctx: &'a mut dyn BlackboardContext,
}
impl BlackboardWrapper<'_> {
    pub fn provides<T: 'static, Z: FnOnce(fn() -> T) + 'static>(
        &mut self,
        key: &str,
        default: Z,
    ) -> Result<ProviderRc<T>, Error> {
        let t = self
            .ctx
            .provides(&TypeId::of::<T>(), key, Box::new(|| Box::new(default)));
        let boxed_rc = t
            .downcast::<Box<ProviderRc<T>>>()
            .or_else(|_| Err("could not downcast"))?;
        Ok(**boxed_rc)
    }
    pub fn consumes<T: 'static>(&mut self, key: &str) -> Result<ConsumerRc<T>, Error> {
        let t = self.ctx.consumes(&TypeId::of::<T>(), key);
        let boxed_rc = t
            .downcast::<Box<ConsumerRc<T>>>()
            .or_else(|_| Err("could not downcast"))?;
        Ok(**boxed_rc)
    }
    // pub fn provides_consumes<T: 'static >(&mut self, key: &str, default: &dyn FnOnce(fn() -> T)) -> Result<ProviderConsumerRc<T>, Error> {
    // let t = self.ctx.provides_consumes(&TypeId::of::<T>(), key);
    // let boxed_rc = t.downcast::<Box<ProviderConsumerRc<T>>>().or_else(|_|Err("could not downcast"))?;
    // Ok(**boxed_rc)
    // }
}

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
    fn setup(&mut self, _ctx: &mut dyn BlackboardContext) -> Result<(), Error> {
        Ok(())
    }
}
