// A simple implementation of the Tree.
use crate::prelude::*;
use std::collections::HashMap;

use crate::{
    BetulaError, Blackboard, BlackboardId, Node, NodeError, NodeId, NodePort, NodeStatus,
    PortConnection, PortDirection, PortName,
};

struct TreeContext<'a> {
    this_node: NodeId,
    tree: &'a BasicTree,
}
impl RunContext for TreeContext<'_> {
    fn children(&self) -> usize {
        self.tree
            .children(self.this_node)
            .expect("node must exist in tree")
            .len()
    }
    fn run(&self, index: usize) -> Result<NodeStatus, NodeError> {
        let ids = self.tree.children(self.this_node)?;
        self.tree.execute(ids[index])
    }
}

use std::cell::RefCell;
use std::collections::HashSet;
#[derive(Debug)]
struct BasicTreeNode {
    node: RefCell<Box<dyn Node>>,
    children: Vec<NodeId>,
}
#[derive(Debug)]
struct BasicBlackboardEntry {
    blackboard: RefCell<Box<dyn Blackboard>>,
    connections: HashSet<PortConnection>,
}
#[derive(Debug, Default)]
pub struct BasicTree {
    nodes: HashMap<NodeId, BasicTreeNode>,
    blackboards: HashMap<BlackboardId, BasicBlackboardEntry>,
    tree_roots: Vec<NodeId>,
}

impl BasicTree {
    pub fn new() -> Self {
        BasicTree {
            nodes: Default::default(),
            blackboards: Default::default(),
            tree_roots: Default::default(),
        }
    }

    fn node_port_connections(
        &self,
        node_port: &NodePort,
    ) -> Result<Vec<PortConnection>, BetulaError> {
        let mut v = vec![];
        for b in self.blackboards.values() {
            v.extend(
                b.connections
                    .iter()
                    .filter(|z| z.node == *node_port)
                    .cloned(),
            );
        }
        Ok(v)
    }
}

impl Tree for BasicTree {
    fn nodes(&self) -> Vec<NodeId> {
        self.nodes.keys().copied().collect()
    }
    fn node_ref(&self, id: NodeId) -> Option<&RefCell<Box<dyn Node>>> {
        Some(&self.nodes.get(&id)?.node)
    }
    fn node_mut(&mut self, id: NodeId) -> Option<&mut dyn Node> {
        let m = self.nodes.get_mut(&id)?;
        Some(&mut **m.node.get_mut())
    }
    fn remove_node(&mut self, id: NodeId) -> Result<Box<dyn Node>, BetulaError> {
        for (_k, v) in self.nodes.iter_mut() {
            v.children.retain(|&x| x != id);
        }
        let value = self
            .nodes
            .remove(&id)
            .ok_or_else(|| -> BetulaError { format!("id {id:?} is not present").into() })?;
        Ok(value.node.into_inner())
    }

    fn add_node_boxed(&mut self, id: NodeId, node: Box<dyn Node>) -> Result<NodeId, BetulaError> {
        self.nodes.insert(
            id,
            BasicTreeNode {
                node: node.into(),
                children: vec![],
            },
        );
        Ok(id)
    }

    fn children(&self, id: NodeId) -> Result<Vec<NodeId>, BetulaError> {
        self.nodes
            .get(&id)
            .ok_or_else(|| format!("node {id:?} is not present").into())
            .map(|x| x.children.clone())
    }

    fn set_children(&mut self, parent: NodeId, children: &[NodeId]) -> Result<(), BetulaError> {
        let n = self
            .nodes
            .get_mut(&parent)
            .ok_or_else(|| format!("node {parent:?} is not present").to_string())?;
        n.children = children.to_vec();
        Ok(())
    }

    fn execute(&self, id: NodeId) -> Result<NodeStatus, NodeError> {
        let mut n = self
            .nodes
            .get(&id)
            .ok_or_else(|| format!("node {id:?} does not exist").to_string())?
            .node
            .try_borrow_mut()?;
        let mut context = TreeContext {
            this_node: id,
            tree: &self,
        };

        n.tick(&mut context)
    }

    fn blackboards(&self) -> Vec<BlackboardId> {
        self.blackboards.keys().copied().collect()
    }

    fn blackboard_ref(&self, id: BlackboardId) -> Option<&std::cell::RefCell<Box<dyn Blackboard>>> {
        Some(&self.blackboards.get(&id)?.blackboard)
    }
    fn blackboard_mut(&mut self, id: BlackboardId) -> Option<&mut dyn Blackboard> {
        let m = self.blackboards.get_mut(&id)?;
        Some(&mut **m.blackboard.get_mut())
    }

    fn add_blackboard_boxed(
        &mut self,
        id: BlackboardId,
        blackboard: Box<dyn Blackboard>,
    ) -> Result<BlackboardId, BetulaError> {
        self.blackboards.insert(
            id,
            BasicBlackboardEntry {
                blackboard: blackboard.into(),
                connections: Default::default(),
            },
        );
        Ok(id)
    }
    fn remove_blackboard(&mut self, id: BlackboardId) -> Option<Box<dyn Blackboard>> {
        // First, disconnect all connections.
        let connections = self.blackboards.get(&id)?.connections.clone();
        for connection in &connections {
            let _ = self.disconnect_port(&connection).ok()?;
        }
        // Then remove the blackboard and return.
        self.blackboards
            .remove(&id)
            .map(|v| v.blackboard.into_inner())
    }

    fn connect_port(&mut self, connection: &PortConnection) -> Result<(), BetulaError> {
        // If this is an input port to the node, first disconnect anything associated to that.
        if connection.node.direction == PortDirection::Input {
            let connections = self.node_port_connections(&connection.node)?;
            for connection in connections {
                self.disconnect_port(&connection)?;
            }
        }
        // node_port: &NodePort,
        // blackboard_port: &BlackboardPort,
        let blackboard_id = connection.blackboard.blackboard();
        let blackboard = self
            .blackboards
            .get_mut(&blackboard_id)
            .ok_or_else(|| format!("blackboard {blackboard_id:?} does not exist").to_string())?;
        let mut blackboard_mut = blackboard.blackboard.try_borrow_mut()?;
        let node_id = connection.node.node();
        let node = self
            .nodes
            .get(&node_id)
            .ok_or_else(|| format!("node {node_id:?} does not exist").to_string())?;
        let mut node_mut = node.node.try_borrow_mut()?;

        struct Remapper<'a, 'b> {
            new_name: &'a PortName,
            blackboard: &'b mut dyn Blackboard,
        }
        impl<'a, 'b> BlackboardInterface for Remapper<'a, 'b> {
            fn writer(
                &mut self,
                id: TypeId,
                key: &PortName,
                default: ValueCreator,
            ) -> Result<Write, NodeError> {
                let _ = key;
                self.blackboard.writer(id, self.new_name, default)
            }

            fn reader(&mut self, id: &TypeId, key: &PortName) -> Result<Read, NodeError> {
                let _ = key;
                self.blackboard.reader(id, self.new_name)
            }
        }
        let blackboard_name = connection.blackboard.name();
        let mut remapped_interface = Remapper {
            new_name: &blackboard_name,
            blackboard: &mut **blackboard_mut,
        };

        let r = node_mut.port_setup(
            &connection.node.name(),
            connection.node.direction,
            &mut remapped_interface,
        );
        if r.is_ok() {
            // Connection was added.
            blackboard.connections.insert(connection.clone());
        }
        r
    }

    fn disconnect_port(&mut self, connection: &PortConnection) -> Result<(), BetulaError> {
        let node_id = connection.node.node();
        let node = self
            .nodes
            .get(&node_id)
            .ok_or_else(|| format!("node {node_id:?} does not exist").to_string())?;
        let mut node_mut = node.node.try_borrow_mut()?;

        let blackboard_id = connection.blackboard.blackboard();
        let blackboard = self
            .blackboards
            .get_mut(&blackboard_id)
            .ok_or_else(|| format!("blackboard {blackboard_id:?} does not exist").to_string())?;

        struct Disconnecter {}
        impl BlackboardInterface for Disconnecter {
            fn writer(
                &mut self,
                id: TypeId,
                key: &PortName,
                default: ValueCreator,
            ) -> Result<Write, NodeError> {
                let _ = (id, key, default);
                let v = |_| Err(format!("writing to disconnected port").into());
                Ok(Box::new(v))
            }

            fn reader(&mut self, id: &TypeId, key: &PortName) -> Result<Read, NodeError> {
                let _ = (id, key);
                let v = || Err(format!("reading from disconnected port").into());
                Ok(Box::new(v))
            }
        }

        let mut remapped_interface = Disconnecter {};
        let r = node_mut.port_setup(
            &connection.node.name(),
            connection.node.direction,
            &mut remapped_interface,
        );
        if r.is_ok() {
            // Connection was added.
            blackboard.connections.remove(connection);
        }
        r
    }

    fn blackboard_connections(&self, id: BlackboardId) -> Vec<PortConnection> {
        self.blackboards
            .get(&id)
            .map(|v| v.connections.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default()
    }

    fn roots(&self) -> Vec<NodeId> {
        self.tree_roots.clone()
    }

    /// Set the roots of this tree.
    fn set_roots(&mut self, new_roots: &[NodeId]) -> Result<(), BetulaError> {
        // Verify that we have these nodes.
        let nodes = self.nodes();
        for n in new_roots {
            if !nodes.contains(&n) {
                return Err(format!("node {n:?} not present").into());
            }
        }
        self.tree_roots = new_roots.to_vec();
        Ok(())
    }
}

use std::any::Any;
use std::rc::Rc;

use crate::blackboard::{BlackboardInterface, Read, Value, ValueCreator, Write};

use std::any::TypeId;
#[derive(Default, Debug)]
pub struct BasicBlackboard {
    values: HashMap<PortName, (TypeId, Rc<RefCell<Value>>)>,
}
use crate::as_any::AsAny;
impl BlackboardInterface for BasicBlackboard {
    fn writer(
        &mut self,
        id: TypeId,
        key: &PortName,
        default: ValueCreator,
    ) -> Result<Write, NodeError> {
        let (typeid, rc) = self
            .values
            .entry(key.clone())
            .or_insert_with(|| (id, Rc::new(RefCell::new(default()))))
            .clone();
        let temp_rc = rc.clone();
        let current_type = {
            let z = temp_rc
                .try_borrow_mut()
                .or_else(|_| Err(format!("{key:?} was already borrowed")))?;
            (**z).as_any_type_name().to_string()
        };
        let owned_key = key.to_string();
        if typeid != id {
            Err(format!(
                "new writer for '{key:?}', has wrong type: already got {}",
                current_type
            )
            .into())
        } else {
            Ok(Box::new(move |v: Value| {
                let mut locked = rc.try_borrow_mut()?;
                if (**locked).as_any_type_id() != (*v).as_any_type_id() {
                    Err(format!(
                        "assignment for '{owned_key:?}' is incorrect type {} expected {}",
                        (**locked).as_any_type_name(),
                        (*v).as_any_type_name()
                    )
                    .into())
                } else {
                    *locked = v;
                    Ok(())
                }
            }))
        }
    }

    fn reader(&mut self, id: &TypeId, key: &PortName) -> Result<Read, NodeError> {
        let (typeid, rc) = self
            .values
            .get(key)
            .ok_or_else(|| format!("key '{key:?}' not found"))?;
        let v = rc.clone();
        if typeid != id {
            Err(format!(
                "new reader for '{key:?}' mismatches type: already got {}",
                rc.as_any_type_name()
            )
            .into())
        } else {
            Ok(Box::new(move || {
                let locked = v.try_borrow_mut()?;
                let cloned = (*locked).clone();
                Ok(cloned)
            }))
        }
    }
}
impl Blackboard for BasicBlackboard {
    fn ports(&self) -> Vec<PortName> {
        self.values.keys().map(|v| v.clone()).collect::<Vec<_>>()
    }

    fn clear(&mut self) {
        self.values.clear()
    }

    fn get(&self, port: &PortName) -> Option<Value> {
        self.values.get(&port).map(|x| x.1.borrow().clone_boxed())
    }

    fn set(&mut self, port: &PortName, value: Value) -> Result<(), BetulaError> {
        let new_value_type = (*value).as_any_type_id();
        let old_value_type = self.values.get(&port).map(|x| (*x).type_id());
        if let Some(old_value_type) = old_value_type {
            if new_value_type != old_value_type {
                return Err("different type already on blackboard".into());
            }
        }
        match self.values.entry(port.clone()) {
            std::collections::hash_map::Entry::Occupied(e) => {
                *(e.get().1.try_borrow_mut()?) = value;
            }
            std::collections::hash_map::Entry::Vacant(v) => {
                v.insert((new_value_type, Rc::new(RefCell::new(value))));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::*;

    #[test]
    fn sequence_fail() -> Result<(), NodeError> {
        let mut tree = BasicTree::new();
        let root = tree.add_node_boxed(NodeId(crate::Uuid::new_v4()), Box::new(SequenceNode {}))?;
        let f1 = tree.add_node_boxed(NodeId(crate::Uuid::new_v4()), Box::new(FailureNode {}))?;
        tree.set_children(root, &vec![f1])?;
        let res = tree.execute(root)?;
        assert_eq!(res, NodeStatus::Failure);
        Ok(())
    }

    #[test]
    fn fallback_success() -> Result<(), NodeError> {
        let mut tree = BasicTree::new();
        let root = tree.add_node_boxed(NodeId(crate::Uuid::new_v4()), Box::new(SelectorNode {}))?;
        let f1 = tree.add_node_boxed(NodeId(crate::Uuid::new_v4()), Box::new(FailureNode {}))?;
        let s1 = tree.add_node_boxed(NodeId(crate::Uuid::new_v4()), Box::new(SuccessNode {}))?;
        tree.set_children(root, &vec![f1, s1])?;
        let res = tree.execute(root)?;
        assert_eq!(res, NodeStatus::Success);
        Ok(())
    }

    #[test]
    fn blackboard_output() {
        let mut bb = BasicBlackboard::default();

        // let mut w = crate::BlackboardContext::new(&mut bb);
        let v_in = 3i64;
        let p = bb.output(&"value".into(), v_in);
        let c = bb.input::<i64>(&"value".into());
        assert!(c.is_ok());
        let c = c.unwrap();
        let v = c.get();
        println!("v: {v:?}");
        assert!(v.is_ok());
        let v = v.unwrap();
        assert_eq!(v_in, v);

        println!("P: {p:?}");
        assert!(p.is_ok());
        let p = p.unwrap();
        let res = p.set(5);
        assert!(res.is_ok());
        let z = bb.output(&"value".into(), 3.3f64);
        println!("z: {z:?}");
        assert!(z.is_err());
        // println!("BasicBlackboard: {bb:?}");
        // let r = bb.consumes(&TypeId::of::<i64>(), "value");
        // assert!(r.is_ok());
        // println!("value: {:?}", r.unwrap()());
        let c = bb.input::<i64>(&"value".into());
        println!("c: {c:?}");
        println!("value: {:?}", z);
    }

    use crate::{Input, NodeType, Output, Port, PortDirection, PortName};

    #[derive(Debug, Default)]
    pub struct OutputNode {
        a_output: Output<f64>,
    }
    impl Node for OutputNode {
        fn tick(&mut self, _ctx: &dyn RunContext) -> Result<NodeStatus, NodeError> {
            self.a_output.set(3.3f64)?;
            Ok(NodeStatus::Success)
        }
        fn ports(&self) -> Result<Vec<Port>, NodeError> {
            Ok(vec![Port::output::<f64>("a")])
        }
        fn port_setup(
            &mut self,
            port: &PortName,
            direction: PortDirection,
            interface: &mut dyn BlackboardInterface,
        ) -> Result<(), NodeError> {
            let _ = direction;
            let z = interface.output::<f64>(&port, 0.0)?;
            self.a_output = z;
            Ok(())
        }

        fn static_type() -> NodeType
        where
            Self: Sized,
        {
            "output_node".into()
        }

        fn node_type(&self) -> NodeType {
            Self::static_type()
        }
    }
    #[derive(Debug, Default)]
    pub struct InputNode {
        a_input: Input<f64>,
    }
    impl Node for InputNode {
        fn tick(&mut self, _ctx: &dyn RunContext) -> Result<NodeStatus, NodeError> {
            let value = self.a_input.get()?;
            if value != 0.0 {
                Ok(NodeStatus::Success)
            } else {
                Ok(NodeStatus::Failure)
            }
        }
        fn ports(&self) -> Result<Vec<Port>, NodeError> {
            Ok(vec![Port::input::<f64>("a")])
        }
        fn port_setup(
            &mut self,
            port: &PortName,
            direction: PortDirection,
            interface: &mut dyn BlackboardInterface,
        ) -> Result<(), NodeError> {
            let _ = direction;
            self.a_input = interface.input::<f64>(&port)?;
            Ok(())
        }

        fn static_type() -> NodeType
        where
            Self: Sized,
        {
            "input_node".into()
        }

        fn node_type(&self) -> NodeType {
            Self::static_type()
        }
    }

    #[test]
    fn test_input_output() -> Result<(), NodeError> {
        // use crate::blackboard::Chalkable;
        let mut tree: Box<dyn Tree> = Box::new(BasicTree::new());
        let root = tree.add_node_boxed(NodeId(crate::Uuid::new_v4()), Box::new(SequenceNode {}))?;
        let o1 = tree.add_node_boxed(
            NodeId(crate::Uuid::new_v4()),
            Box::new(OutputNode::default()),
        )?;
        let i1 = tree.add_node_boxed(
            NodeId(crate::Uuid::new_v4()),
            Box::new(InputNode::default()),
        )?;
        tree.set_children(root, &vec![o1, i1])?;

        // Add the blackboard.
        let bb = tree.add_blackboard_boxed(
            BlackboardId(crate::Uuid::new_v4()),
            Box::new(BasicBlackboard::default()),
        )?;

        let output_ports = tree.node_ports(o1)?;
        tree.connect_port_to_blackboard(&output_ports[0], bb)?;
        let input_ports = tree.node_ports(i1)?;
        tree.connect_port_to_blackboard(&input_ports[0], bb)?;

        let res = tree.execute(root)?;
        assert_eq!(res, NodeStatus::Success);

        // get the value from the blackboard.
        let bbref = tree.blackboard_ref(bb).unwrap();
        let entries = bbref.borrow().ports();
        assert_eq!(entries.len(), 1);
        let value = bbref.borrow().get(&entries[0]);
        println!("Value: {value:?}, {}", value.as_any_type_name());
        let expected: Option<Box<dyn crate::blackboard::Chalkable>> = Some(Box::new(3.3f64));
        println!("expected: {expected:?}, {}", expected.as_any_type_name());
        assert!(value.unwrap().is_equal(&*expected.unwrap()));
        Ok(())
    }
}
