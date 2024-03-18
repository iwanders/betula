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
    pub use crate::{Consumer, Error, Node, Provider, RunContext, Status};
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

use std::cell::Cell;
use std::cell::RefCell;

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

use std::any::{Any, TypeId};
use std::rc::Rc;

/// Function type for allocating storage on the blackboard.
pub type BlackboardValueCreator = Box<dyn FnOnce() -> BlackboardValue>;

pub trait ClonableBlackboardValue: std::any::Any + std::fmt::Debug {
    fn clone_boxed<'a>(&self) -> Box<dyn ClonableBlackboardValue>;
}

impl<T> ClonableBlackboardValue for T
where
    T: std::any::Any + Clone + 'static + std::fmt::Debug,
{
    fn clone_boxed(&self) -> Box<dyn ClonableBlackboardValue> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn ClonableBlackboardValue> {
    fn clone(&self) -> Self {
        self.clone_boxed()
    }
}

#[derive(Debug, Clone)]
pub enum BlackboardValue {
    Small((TypeId, [u8; 8])),
    Big(Box<dyn ClonableBlackboardValue>),
}

impl From<i64> for BlackboardValue {
    fn from(v: i64) -> Self {
        BlackboardValue::Small((TypeId::of::<i64>(), v.to_ne_bytes()))
    }
}
impl TryInto<i64> for BlackboardValue {
    type Error = Box<dyn std::error::Error + Send + Sync>;
    fn try_into(self) -> Result<i64, <Self as TryInto<i64>>::Error> {
        if let BlackboardValue::Small((tid, bytes)) = self {
            if tid == TypeId::of::<i64>() {
                Ok(i64::from_ne_bytes(bytes))
            } else {
                Err("incorrect type".into())
            }
        } else {
            Err("not a small value".into())
        }
    }
}

impl From<f64> for BlackboardValue {
    fn from(v: f64) -> Self {
        BlackboardValue::Small((TypeId::of::<f64>(), v.to_ne_bytes()))
    }
}

pub type BlackboardRead = Box<dyn Fn() -> Result<BlackboardValue, Error>>;
pub type BlackboardWrite = Box<dyn Fn(BlackboardValue) -> Result<(), Error>>;

/// The blackboard is a bit clunky at the moment. I don't see a way to:
///   * Avoid exposing Rc<RefCell<Box<dyn Any>>>
///   * or avoid passing values as Box<dyn Any>, which would lead to many allocations.
///
/// Interface to a blackboard, this is only necessary during setup, it should
/// return Rc<RefCell<Box<dyn Any>>> types.
///
/// Convenience wrapper is provided with BlackboardContext to make it easier
/// to setup the appropriate providers and subscribers.
///
/// We do this dance because we can't have generic methods on an object
/// safe trait, but with this whole construct we can still provide different
/// implementing types for the actual storage of the values, as long as they
/// implement the BlackboardInterface. While still getting type checking and
/// all that.
pub trait BlackboardInterface {
    fn provides(
        &mut self,
        id: TypeId,
        key: &str,
        default: BlackboardValueCreator,
    ) -> Result<BlackboardWrite, Error>;

    fn consumes(&mut self, id: &TypeId, key: &str) -> Result<BlackboardRead, Error>;
    // fn provides_consumes(&mut self, id: &TypeId, key: &str, default: BlackboardValueCreator) -> Box<dyn std::any::Any>;
}

/// The boxed trait that nodes should use to provide values to the blackboard.
pub type Provider<T> = Box<dyn ProviderTrait<ProviderItem = T>>;

/// The boxed trait that nodes should use to consume values from the blackboard.
pub type Consumer<T> = Box<dyn ConsumerTrait<ConsumerItem = T>>;

/// The boxed trait that nodes should use to provide and consume values from the blackboard.
pub type ProviderConsumer<T> = Box<dyn ProviderConsumerTrait<ProviderItem = T, ConsumerItem = T>>;

/// Wrapper type to make it easier to setup the appropriate providers and
/// consumers from the blackboard interface.
pub struct BlackboardContext<'a> {
    ctx: &'a mut dyn BlackboardInterface,
}

impl<'a> BlackboardContext<'a> {
    pub fn new(ctx: &'a mut dyn BlackboardInterface) -> BlackboardContext {
        Self { ctx }
    }

    pub fn interface(&mut self) -> &&'a mut dyn BlackboardInterface {
        &self.ctx
    }

    pub fn provides<T: 'static + std::convert::Into<BlackboardValue>>(
        &mut self,
        key: &str,
        default: T,
    ) -> Result<Provider<T>, Error>
    where
        BlackboardValue: From<T>,
    {
        self.provides_or_else::<T, _>(key, || default.into())
    }

    pub fn provides_or_else<T: 'static, Z: FnOnce() -> BlackboardValue + 'static>(
        &mut self,
        key: &str,
        default: Z,
    ) -> Result<Provider<T>, Error>
    where
        BlackboardValue: From<T>,
    {
        let writer = self
            .ctx
            .provides(TypeId::of::<T>(), key, Box::new(default))?;
        // t gave back a Rc<RefCell<Box<dyn Any>>>
        // Now we need to make our Provider for this type.
        struct ProviderFor<TT>
        where
            BlackboardValue: From<TT>,
        {
            key: String,
            type_name: String,
            z: std::marker::PhantomData<TT>,
            writer: BlackboardWrite,
        }
        impl<TT: 'static> ProviderTrait for ProviderFor<TT>
        where
            BlackboardValue: From<TT>,
        {
            type ProviderItem = TT;
            fn set(&self, v: Self::ProviderItem) -> Result<(), Error> {
                let bb_value = v.into();
                (self.writer)(bb_value)
            }
        }

        impl<TT: 'static> std::fmt::Debug for ProviderFor<TT>
        where
            BlackboardValue: From<TT>,
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
                write!(f, "Provider::<{}>(\"{}\")", self.type_name, self.key)
            }
        }

        Ok(Box::new(ProviderFor::<T> {
            writer,
            z: std::marker::PhantomData,
            type_name: std::any::type_name::<T>().to_string(),
            key: key.to_string(),
        }))
    }
    // fn consumes(&mut self, id: &TypeId, key: &str) ->  Result<BlackboardRead, Error>;
    pub fn consumes<T: 'static>(&mut self, key: &str) -> Result<Consumer<T>, Error>
    where
        BlackboardValue: TryInto<T>,
    {
        let reader = self.ctx.consumes(&TypeId::of::<T>(), key)?;

        struct ConsumerFor<TT>
        where
            BlackboardValue: TryInto<TT>,
        {
            key: String,
            type_name: String,
            z: std::marker::PhantomData<TT>,
            reader: BlackboardRead,
        }
        impl<TT: 'static> ConsumerTrait for ConsumerFor<TT>
        where
            BlackboardValue: TryInto<TT>,
        {
            type ConsumerItem = TT;
            fn get(&self) -> Result<TT, Error> {
                Ok((self.reader)()?.try_into().ok().expect("testing"))
            }
        }

        impl<TT: 'static> std::fmt::Debug for ConsumerFor<TT>
        where
            BlackboardValue: TryInto<TT>,
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
                write!(f, "Consumer::<{}>(\"{}\")", self.type_name, self.key)
            }
        }

        Ok(Box::new(ConsumerFor::<T> {
            reader,
            z: std::marker::PhantomData,
            type_name: std::any::type_name::<T>().to_string(),
            key: key.to_string(),
        }))
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
    fn setup(&mut self, _ctx: &mut BlackboardContext) -> Result<(), Error> {
        Ok(())
    }
}
