use crate::as_any::{AsAny, AsAnyHelper};
use crate::{BetulaError, NodeError, NodeId, Uuid};
use serde::{Deserialize, Serialize};
use std::any::{Any, TypeId};

/// Id for blackboards.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
pub struct BlackboardId(pub Uuid);

/// A name for an input or outuput port.
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
pub struct PortName(pub String);
impl PortName {
    pub fn as_ref(&self) -> &str {
        &self.0
    }
}
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
#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum PortDirection {
    /// The port consumes the value, it is an input to the node.
    Input,
    /// The port provides the value, it is an output from the node.
    Output,
}

/// A port for a node.
///
/// Ports have a name, direction and type.
#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct Port {
    port_type: PortType,
    direction: PortDirection,
    name: PortName,
}

impl Port {
    pub fn input<T: 'static>(name: impl Into<PortName>) -> Self {
        Port {
            port_type: PortType::new::<T>(),
            direction: PortDirection::Input,
            name: name.into(),
        }
    }
    pub fn output<T: 'static>(name: impl Into<PortName>) -> Self {
        Port {
            port_type: PortType::new::<T>(),
            direction: PortDirection::Output,
            name: name.into(),
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

/// An untyped identifier for a specific node's port.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
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

/// An untyped identifier for a port on the blackboard.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
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
    pub fn set_name(&mut self, new_name: &PortName) {
        self.name = new_name.clone();
    }
}

/// A port connection is an untyped connection between a [`NodePort`] and [`BlackboardPort`].
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
pub struct PortConnection {
    pub node: NodePort,
    pub blackboard: BlackboardPort,
}
impl PortConnection {
    pub fn new(node: NodePort, blackboard: BlackboardPort) -> Self {
        Self { node, blackboard }
    }
    pub fn blackboard_id(&self) -> BlackboardId {
        self.blackboard.blackboard()
    }
}

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

/// Requirements for any value that is written to the blackboard.
/// Clone, std::any::Any, std::fmt::Debug, std::cmp::PartialEq
pub trait Chalkable: std::fmt::Debug + crate::AsAny + Send {
    fn clone_boxed(&self) -> Box<dyn Chalkable>;
    fn is_equal(&self, other: &dyn Chalkable) -> bool;
}

impl<T> Chalkable for T
where
    T: Clone + 'static + std::fmt::Debug + Any + std::cmp::PartialEq + Send,
{
    fn clone_boxed(&self) -> Box<dyn Chalkable> {
        Box::new(self.clone())
    }

    fn is_equal(&self, other: &dyn Chalkable) -> bool {
        // println!("eq: {self:?}, {other:?}");
        // println!("eq: {:?}   {:?}", self.as_any_ref().type_id(), other.as_any_ref().type_id());
        if self.as_any_ref().type_id() != other.as_any_ref().type_id() {
            false
        } else {
            let left = self.downcast_ref::<T>();
            let right = other.downcast_ref::<T>();
            if left.is_none() || right.is_none() {
                return false;
            }
            let left = left.unwrap();
            let right = right.unwrap();
            // println!("leftright: {left:?}, {right:?}");
            std::cmp::PartialEq::eq(left, right)
        }
    }
}

impl Clone for Box<dyn Chalkable> {
    fn clone(&self) -> Self {
        (**self).clone_boxed()
    }
}

/// A value on the blackboard.
pub type Value = Box<dyn Chalkable>;

/// Function type for allocating storage on the blackboard.
pub type ValueCreator = Box<dyn Fn() -> Value>;

/// Boxed function to read values from the blackboard.
pub type Read = Box<dyn Fn() -> Result<Value, NodeError>>;

/// Boxed function to write values to the blackboard. Deliberately does NOT
/// return the previous value to ensure purity.
pub type Write = Box<dyn Fn(Value) -> Result<(), NodeError>>;

/// The object safe blackboard output interface, provides [`Write`] functions.
///
/// Don't interact with this directly, do so through [`SetupOutput`].
pub trait BlackboardOutputInterface {
    fn writer(
        &mut self,
        id: TypeId,
        key: &PortName,
        default: &ValueCreator,
    ) -> Result<Write, NodeError>;
}

/// The object safe blackboard output interface, provides [`Read`] functions.
///
/// Don't interact with this directly, do so through [`SetupInput`].
pub trait BlackboardInputInterface {
    fn reader(&mut self, id: &TypeId, key: &PortName) -> Result<Read, NodeError>;
}

/// Interface blackboards must provide.
pub trait Blackboard:
    std::fmt::Debug + AsAny + BlackboardOutputInterface + BlackboardInputInterface
{
    /// Create a new instance of this blackboard.
    fn new() -> Self
    where
        Self: Sized;

    /// The current ports on this blackboard.
    fn ports(&self) -> Vec<PortName>;

    /// Clear the entire blackboard, ports is empty after this.
    fn clear(&mut self);

    /// Get the value for the provided port name.
    fn get(&self, port: &PortName) -> Option<Value>;

    /// Set a value to the provided port name.
    ///
    /// Returns `Err` if the type that already exists on the blackboard is different
    /// from the type of the value that is provided.
    fn set(&mut self, port: &PortName, value: Value) -> Result<(), BetulaError>;
}

/// Helper trait to interact with [`BlackboardOutputInterface`].
///
/// See [`crate::Node::setup_outputs`] for an example.
pub trait SetupOutput: BlackboardOutputInterface {
    fn output<T: 'static + Chalkable + Clone>(
        &mut self,
        key: impl Into<PortName>,
        default: T,
    ) -> Result<Output<T>, NodeError> {
        let x: ValueCreator = Box::new(move || Box::new(default.clone()));
        self.output_or_else::<T, _>(key, x)
    }

    fn output_or_else<T: 'static + Chalkable + Clone, Z: Fn() -> Value + 'static>(
        &mut self,
        key: impl Into<PortName>,
        default_maker: Z,
    ) -> Result<Output<T>, NodeError> {
        let key: PortName = key.into();
        let x: ValueCreator = Box::new(default_maker);
        let writer = BlackboardOutputInterface::writer(self, TypeId::of::<T>(), &key, &x)?;
        struct OutputFor<TT> {
            key: PortName,
            type_name: String,
            z: std::marker::PhantomData<TT>,
            writer: Write,
        }
        impl<TT: 'static + Chalkable + Clone> OutputTrait for OutputFor<TT> {
            type OutputItem = TT;
            fn set(&self, v: Self::OutputItem) -> Result<(), NodeError> {
                let z = Box::new(v);
                (self.writer)(z)
            }
        }

        impl<TT: 'static + Chalkable + Clone> std::fmt::Debug for OutputFor<TT> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
                write!(f, "Output::<{}>(\"{:?}\")", self.type_name, self.key)
            }
        }

        Ok(Box::new(OutputFor::<T> {
            writer,
            z: std::marker::PhantomData,
            type_name: std::any::type_name::<T>().to_string(),
            key: key.clone(),
        }))
    }
}

impl<T: BlackboardOutputInterface> SetupOutput for T {}
impl SetupOutput for dyn BlackboardOutputInterface + '_ {}

/// Helper trait to interact with [`BlackboardInputInterface`].
///
/// See [`crate::Node::setup_inputs`] for an example.
pub trait SetupInput: BlackboardInputInterface {
    fn input<T: 'static + Chalkable + Clone>(
        &mut self,
        key: impl Into<PortName>,
    ) -> Result<Input<T>, NodeError> {
        let key: PortName = key.into();
        let reader = BlackboardInputInterface::reader(self, &TypeId::of::<T>(), &key)?;

        struct InputFor<TT> {
            key: PortName,
            type_name: String,
            z: std::marker::PhantomData<TT>,
            reader: Read,
        }
        impl<TT: 'static + Chalkable + Clone> InputTrait for InputFor<TT> {
            type InputItem = TT;
            fn get(&self) -> Result<TT, NodeError> {
                let boxed_value = (self.reader)()?;
                let v = (*boxed_value).downcast_ref::<TT>().ok_or_else(|| {
                    format!(
                        "could not downcast {:?} to {:?}",
                        (*boxed_value).as_any_type_name(),
                        std::any::type_name::<TT>()
                    )
                })?;
                Ok((*v).clone())
            }
        }

        impl<TT: 'static + Chalkable + Clone> std::fmt::Debug for InputFor<TT> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
                write!(f, "Input::<{}>(\"{:?}\")", self.type_name, self.key)
            }
        }

        Ok(Box::new(InputFor::<T> {
            reader,
            z: std::marker::PhantomData,
            type_name: std::any::type_name::<T>().to_string(),
            key: key.clone(),
        }))
    }
}
impl<T: BlackboardInputInterface> SetupInput for T {}
impl SetupInput for dyn BlackboardInputInterface + '_ {}

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
        assert!(!a.is_equal(&*b));
        // println!("a_eq_b: {a_eq_b:?}");
        assert!(a.is_equal(&*c));

        #[derive(Debug, Clone, PartialEq)]
        struct Z(f32);
        let a: Box<dyn Chalkable> = Box::new(Z(3.0f32));
        let b: Box<dyn Chalkable> = Box::new(Z(5.0f32));
        let c = a.clone();
        println!("a: {a:?}");
        println!("c cloned a: {c:?}");
        assert!(!a.is_equal(&*b));
        // println!("a_eq_b: {a_eq_b:?}");
        assert!(a.is_equal(&*c));

        assert!(std::cmp::PartialEq::eq(&3.3f64, &3.3f64));

        let a: Box<dyn Chalkable> = Box::new(3.3f64);
        let b: Box<dyn Chalkable> = Box::new(3.3f64);

        assert!(a.is_equal(&*b));
    }
}
