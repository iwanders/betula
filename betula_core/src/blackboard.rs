use crate::as_any::{AsAny, AsAnyHelper};
use crate::{Consumer, NodeError, Provider};
use std::any::{Any, TypeId};

/// Requirements for any value that is written to the blackboard.
/// Clone, std::any::Any, std::fmt::Debug, std::cmp::PartialEq
pub trait Chalkable: std::fmt::Debug + crate::AsAny {
    fn clone_boxed(&self) -> Box<dyn Chalkable>;
    fn equality(&self, other: &dyn Chalkable) -> bool;
}

impl<T> Chalkable for T
where
    T: Clone + 'static + std::fmt::Debug + Any + std::cmp::PartialEq,
{
    fn clone_boxed(&self) -> Box<dyn Chalkable> {
        Box::new(self.clone())
    }

    fn equality(&self, other: &dyn Chalkable) -> bool {
        if self.as_any_ref().type_id() != other.as_any_ref().type_id() {
            false
        } else {
            let left = self.downcast_ref::<T>();
            let right = other.downcast_ref::<T>();
            left == right
        }
    }
}

impl Clone for Box<dyn Chalkable> {
    fn clone(&self) -> Self {
        (**self).clone_boxed()
    }
}

impl PartialEq<dyn Chalkable> for dyn Chalkable {
    fn eq(&self, rhs: &(dyn Chalkable + 'static)) -> bool {
        self.equality(rhs)
    }
}

pub type Value = Box<dyn Chalkable>;

/// Function type for allocating storage on the blackboard.
pub type ValueCreator = Box<dyn FnOnce() -> Value>;

/// Boxed function to read values from the blackboard.
pub type Read = Box<dyn Fn() -> Result<Value, NodeError>>;

/// Boxed function to write values to the blackboard. Deliberately does NOT
/// return the previous value to ensure purity.
pub type Write = Box<dyn Fn(Value) -> Result<(), NodeError>>;

use crate::{ConsumerTrait, ProviderTrait};

/// The object safe blackboard interface, providing access to the getters and setters.
/// Interation through BlackboardSetup is very much recommended.
pub trait BlackboardInterface {
    fn writer(&mut self, id: TypeId, key: &str, default: ValueCreator) -> Result<Write, NodeError>;

    fn reader(&mut self, id: &TypeId, key: &str) -> Result<Read, NodeError>;
}

pub trait Setup: BlackboardInterface {
    fn provides<T: 'static + Chalkable + Clone>(
        &mut self,
        key: &str,
        default: T,
    ) -> Result<Provider<T>, NodeError> {
        self.provides_or_else::<T, _>(key, || Box::new(default))
    }

    fn provides_or_else<T: 'static + Chalkable + Clone, Z: FnOnce() -> Value + 'static>(
        &mut self,
        key: &str,
        default_maker: Z,
    ) -> Result<Provider<T>, NodeError> {
        let writer =
            BlackboardInterface::writer(self, TypeId::of::<T>(), key, Box::new(default_maker))?;
        struct ProviderFor<TT> {
            key: String,
            type_name: String,
            z: std::marker::PhantomData<TT>,
            writer: Write,
        }
        impl<TT: 'static + Chalkable + Clone> ProviderTrait for ProviderFor<TT> {
            type ProviderItem = TT;
            fn set(&self, v: Self::ProviderItem) -> Result<(), NodeError> {
                let z = Box::new(v);
                (self.writer)(z)
            }
        }

        impl<TT: 'static + Chalkable + Clone> std::fmt::Debug for ProviderFor<TT> {
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

    fn consumes<T: 'static + Chalkable + Clone>(
        &mut self,
        key: &str,
    ) -> Result<Consumer<T>, NodeError> {
        let reader = BlackboardInterface::reader(self, &TypeId::of::<T>(), key)?;

        struct ConsumerFor<TT> {
            key: String,
            type_name: String,
            z: std::marker::PhantomData<TT>,
            reader: Read,
        }
        impl<TT: 'static + Chalkable + Clone> ConsumerTrait for ConsumerFor<TT> {
            type ConsumerItem = TT;
            fn get(&self) -> Result<TT, NodeError> {
                let boxed_value = (self.reader)()?;
                let v = (*boxed_value).downcast_ref::<TT>().ok_or_else(|| {
                    format!(
                        "could not downcast {:?} to {:?}",
                        boxed_value.type_name(),
                        std::any::type_name::<TT>()
                    )
                })?;
                Ok((*v).clone())
            }
        }

        impl<TT: 'static + Chalkable + Clone> std::fmt::Debug for ConsumerFor<TT> {
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
}

impl<T: BlackboardInterface> Setup for T {}
impl Setup for dyn BlackboardInterface + '_ {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blackboard_reqs() {
        let a: Box<dyn Chalkable> = Box::new(3u8);
        let b: Box<dyn Chalkable> = Box::new(5u8);
        let c = a.clone();
        println!("a: {a:?}");
        println!("c cloned a: {c:?}");
        assert!(a != b);
        // println!("a_eq_b: {a_eq_b:?}");
        assert!(a == c);

        #[derive(Debug, Clone, PartialEq)]
        struct Z(f32);
        let a: Box<dyn Chalkable> = Box::new(Z(3.0f32));
        let b: Box<dyn Chalkable> = Box::new(Z(5.0f32));
        let c = a.clone();
        println!("a: {a:?}");
        println!("c cloned a: {c:?}");
        assert!(a != b);
        // println!("a_eq_b: {a_eq_b:?}");
        assert!(a == c);
    }
}
