use crate::as_any::AsAny;
use std::any::{Any, TypeId};
pub trait Chalkable: Any + std::fmt::Debug + crate::AsAny {
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
        println!(
            "std::any::type_name::<T>().to_string(): {:?}",
            std::any::type_name::<T>().to_string()
        );
        println!("TypeId::of::<T>(): {:?}", TypeId::of::<T>());
        println!("self: {:?}", self.as_any_ref().type_id());
        println!("other: {:?}", other.as_any_ref().type_id());
        if self.as_any_ref().type_id() != other.as_any_ref().type_id() {
            dbg!();
            false
        } else {
            dbg!();
            let left = (self as &dyn Any).downcast_ref::<T>();
            let right = other.as_any_ref().downcast_ref::<T>();
            println!("left: {left:?}");
            println!("right: {right:?}");
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

pub type BlackboardValue = Box<dyn Chalkable>;

// pub trait Chalkable : std::any::Any + std::fmt::Debug + Clone + std::cmp::PartialEq<dyn Chalkable>{}
// pub type BlackboardValueZ = Box<dyn Chalkable>;
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blackboard_reqs() {
        let a: Box<dyn Chalkable> = Box::new(3u8);
        let b: Box<dyn Chalkable> = Box::new(5u8);
        let c = a.clone();
        println!("c: {c:?}");
        // let a_eq_b = a == b;
        // println!("a_eq_b: {a_eq_b:?}");
        let a_eq_c = a == c;
        println!("a_eq_c: {a_eq_c:?}");
    }
}
