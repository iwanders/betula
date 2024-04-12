// A simple implementation of the Tree.
use crate::prelude::*;
use std::collections::HashMap;

use crate::{
    blackboard::{PortConnection, PortDirection, PortName},
    BetulaError, Blackboard, BlackboardId, Node, NodeError, NodeId, NodePort, NodeStatus,
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

    fn node_connections(&self, node_id: &NodeId) -> Result<Vec<PortConnection>, BetulaError> {
        let mut v = vec![];
        for b in self.blackboards.values() {
            v.extend(
                b.connections
                    .iter()
                    .filter(|z| z.node.node() == *node_id)
                    .cloned(),
            );
        }
        Ok(v)
    }

    fn node_input_connections(&self, node: NodeId) -> Result<Vec<PortConnection>, BetulaError> {
        let mut v = vec![];
        for b in self.blackboards.values() {
            v.extend(
                b.connections
                    .iter()
                    .filter(|z| z.node.node() == node && z.node.direction() == PortDirection::Input)
                    .cloned(),
            );
        }
        Ok(v)
    }
    fn node_output_connections(&self, node: NodeId) -> Result<Vec<PortConnection>, BetulaError> {
        let mut v = vec![];
        for b in self.blackboards.values() {
            v.extend(
                b.connections
                    .iter()
                    .filter(|z| {
                        z.node.node() == node && z.node.direction() == PortDirection::Output
                    })
                    .cloned(),
            );
        }
        Ok(v)
    }

    fn setup_node_outputs(
        &self,
        node: NodeId,
        connections: &[PortConnection],
    ) -> Result<(), BetulaError> {
        // Okay, so for all ports that we don't know, we're going to return a dummy function.
        // Outputs can map to multiple blackboards.
        // First check if we only got outputs...
        for c in connections {
            if c.node.direction() == PortDirection::Input {
                return Err(format!("got input port for output setup").into());
            }
        }

        // Assembly the desired functions.
        let mut by_portname: HashMap<PortName, Vec<PortConnection>> = Default::default();
        for connection in connections {
            let z = by_portname
                .entry(connection.node.name().clone())
                .or_default();
            z.push(connection.clone());
        }

        struct Remapper<'a, 'b> {
            by_portname: &'a HashMap<PortName, Vec<PortConnection>>,
            blackboards: &'b HashMap<BlackboardId, BasicBlackboardEntry>,
        }
        impl<'a, 'b> BlackboardOutputInterface for Remapper<'a, 'b> {
            fn writer(
                &mut self,
                id: TypeId,
                key: &PortName,
                default: &ValueCreator,
            ) -> Result<Write, NodeError> {
                let connections = self.by_portname.get(key);
                if let Some(connections) = connections {
                    // Collect the writers from all the blackboards.
                    let mut writers = vec![];
                    for connection in connections {
                        let blackboard_id = connection.blackboard.blackboard();
                        let blackboard_name = connection.blackboard.name();
                        let blackboard = self.blackboards.get(&blackboard_id).ok_or_else(|| {
                            format!("blackboard {blackboard_id:?} does not exist").to_string()
                        })?;
                        let mut blackboard_mut = blackboard.blackboard.try_borrow_mut()?;
                        writers.push(blackboard_mut.writer(id, &blackboard_name, default)?);
                    }
                    let one_setter: Write = Box::new(move |v| {
                        for w in &writers {
                            (*w)(v.clone())?;
                        }
                        Ok(())
                    });
                    Ok(one_setter)
                } else {
                    let v = |_| Err(format!("writing to disconnected port").into());
                    Ok(Box::new(v))
                }
            }
        }

        let mut remapped_interface = Remapper {
            by_portname: &by_portname,
            blackboards: &self.blackboards,
        };

        let node = self
            .nodes
            .get(&node)
            .ok_or_else(|| format!("node {node:?} does not exist").to_string())?;
        let mut node_mut = node.node.try_borrow_mut()?;

        node_mut.setup_outputs(&mut remapped_interface)?;

        Ok(())
    }

    fn setup_node_inputs(
        &self,
        node: NodeId,
        connections: &[PortConnection],
    ) -> Result<(), BetulaError> {
        // Okay, so for all ports that we don't know, we're going to return a dummy function.
        // Outputs can map to multiple blackboards.
        // First check if we only got outputs...
        for c in connections {
            if c.node.direction() == PortDirection::Output {
                return Err(format!("got output port for input setup").into());
            }
        }

        // Assembly the desired functions.
        let mut by_portname: HashMap<PortName, Option<PortConnection>> = Default::default();
        for connection in connections {
            let portname = connection.node.name();
            let z = by_portname.entry(portname.clone()).or_default();
            if z.is_some() {
                return Err(format!("got two inputs for port {portname:?}").into());
            }
            *z = Some(connection.clone());
        }

        struct Remapper<'a, 'b> {
            by_portname: &'a HashMap<PortName, Option<PortConnection>>,
            blackboards: &'b HashMap<BlackboardId, BasicBlackboardEntry>,
        }
        impl<'a, 'b> BlackboardInputInterface for Remapper<'a, 'b> {
            fn reader(&mut self, id: &TypeId, key: &PortName) -> Result<Read, NodeError> {
                let input_connection = self.by_portname.get(key);
                if let Some(found_entry) = input_connection {
                    if let Some(connection) = found_entry {
                        // Collect the writers from all the blackboards.
                        let blackboard_id = connection.blackboard.blackboard();
                        let blackboard_name = connection.blackboard.name();
                        let blackboard = self.blackboards.get(&blackboard_id).ok_or_else(|| {
                            format!("blackboard {blackboard_id:?} does not exist").to_string()
                        })?;
                        let mut blackboard_mut = blackboard.blackboard.try_borrow_mut()?;
                        blackboard_mut.reader(id, &blackboard_name)
                    } else {
                        let v = || Err(format!("reading from disconnected port").into());
                        Ok(Box::new(v))
                    }
                } else {
                    let v = || Err(format!("reading from disconnected port").into());
                    Ok(Box::new(v))
                }
            }
        }

        let mut remapped_interface = Remapper {
            by_portname: &by_portname,
            blackboards: &self.blackboards,
        };

        let node = self
            .nodes
            .get(&node)
            .ok_or_else(|| format!("node {node:?} does not exist").to_string())?;
        let mut node_mut = node.node.try_borrow_mut()?;

        node_mut.setup_inputs(&mut remapped_interface)?;

        Ok(())
    }

    fn disconnect_node_ports(
        &self,
        node_id: &NodeId,
        direction: PortDirection,
    ) -> Result<(), BetulaError> {
        let node = self
            .nodes
            .get(&node_id)
            .ok_or_else(|| format!("node {node_id:?} does not exist").to_string())?;
        let mut node_mut = node.node.try_borrow_mut()?;
        struct Disconnecter {}
        impl BlackboardOutputInterface for Disconnecter {
            fn writer(
                &mut self,
                id: TypeId,
                key: &PortName,
                default: &ValueCreator,
            ) -> Result<Write, NodeError> {
                let _ = (id, key, default);
                let v = |_| Err(format!("writing to disconnected port").into());
                Ok(Box::new(v))
            }
        }
        impl BlackboardInputInterface for Disconnecter {
            fn reader(&mut self, id: &TypeId, key: &PortName) -> Result<Read, NodeError> {
                let _ = (id, key);
                let v = || Err(format!("reading from disconnected port").into());
                Ok(Box::new(v))
            }
        }

        let mut remapped_interface = Disconnecter {};
        if direction == PortDirection::Input {
            node_mut.setup_inputs(&mut remapped_interface)?;
        } else {
            node_mut.setup_outputs(&mut remapped_interface)?;
        }
        Ok(())
    }
}

impl Tree for BasicTree {
    fn new() -> Self {
        BasicTree::new()
    }

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
        // First, disconnect all ports associated to this node.
        for connection in self.node_connections(&id)? {
            self.disconnect_port(&connection)?;
        }

        // then, actually discard this node.
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

    fn remove_blackboard(&mut self, id: BlackboardId) -> Result<Box<dyn Blackboard>, BetulaError> {
        // First, disconnect all connections.
        let blackboard = self
            .blackboards
            .get(&id)
            .ok_or::<BetulaError>(format!("could not find blackboard {id:?}").into())?;
        let connections = blackboard.connections.clone();
        for connection in connections.iter() {
            let _ = self.disconnect_port(&connection)?;
        }
        // Then remove the blackboard and return.
        self.blackboards
            .remove(&id)
            .map(|v| v.blackboard.into_inner())
            .ok_or(format!("could not find blackboard {id:?}").into())
    }

    fn connect_port(&mut self, connection: &PortConnection) -> Result<(), BetulaError> {
        let node_id = connection.node.node();
        if connection.node.direction() == PortDirection::Input {
            // If this is an input port to the node, first disconnect anything associated to that, each node
            // can only have one input afterall.
            let current_connections = self.node_port_connections(&connection.node)?;
            assert!(current_connections.len() <= 1);
            for existing_connection in current_connections {
                self.disconnect_port(&existing_connection)?;
            }

            let node_id = connection.node.node();
            self.disconnect_node_ports(&node_id, PortDirection::Input)?;
            let mut inputs = self.node_input_connections(node_id)?;
            inputs.push(connection.clone());
            self.setup_node_inputs(node_id, &inputs)?;
        } else {
            // Outputs, we can write to multiple blackboards.
            let mut outputs = self.node_output_connections(node_id)?;
            self.disconnect_node_ports(&node_id, PortDirection::Output)?;
            outputs.push(connection.clone());
            self.setup_node_outputs(node_id, &outputs)?;
        }

        // Connection was added.
        let blackboard_id = connection.blackboard.blackboard();
        let blackboard = self
            .blackboards
            .get_mut(&blackboard_id)
            .ok_or_else(|| format!("blackboard {blackboard_id:?} does not exist").to_string())?;
        blackboard.connections.insert(connection.clone());
        Ok(())
    }

    fn disconnect_port(&mut self, connection: &PortConnection) -> Result<(), BetulaError> {
        let node_id = connection.node.node();
        // First disconnect all ports of this direction.
        self.disconnect_node_ports(&node_id, connection.node.direction())?;

        // Disconnect cannot really fail, as the connect can only fail on type mismatches.
        if connection.node.direction() == PortDirection::Input {
            // And then setup the inputs again.
            let mut inputs = self.node_input_connections(node_id)?;
            inputs = inputs.drain(..).filter(|c| *c != *connection).collect();
            self.setup_node_inputs(node_id, &inputs)?;
        } else {
            // And the outputs.
            let mut outputs = self.node_output_connections(node_id)?;
            outputs = outputs.drain(..).filter(|c| *c != *connection).collect();
            self.setup_node_outputs(node_id, &outputs)?;
        }

        let blackboard_id = connection.blackboard.blackboard();
        let blackboard = self
            .blackboards
            .get_mut(&blackboard_id)
            .ok_or_else(|| format!("blackboard {blackboard_id:?} does not exist").to_string())?;
        blackboard.connections.remove(connection);

        Ok(())
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

use crate::blackboard::{
    BlackboardInputInterface, BlackboardOutputInterface, Read, Value, ValueCreator, Write,
};

use std::any::TypeId;
#[derive(Default, Debug)]
pub struct BasicBlackboard {
    values: HashMap<PortName, (TypeId, Rc<RefCell<Value>>)>,
}
use crate::as_any::AsAny;
impl BlackboardOutputInterface for BasicBlackboard {
    fn writer(
        &mut self,
        id: TypeId,
        key: &PortName,
        default: &ValueCreator,
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
}

impl BlackboardInputInterface for BasicBlackboard {
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
    fn new() -> Self {
        Self::default()
    }
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
        let p = bb.output("value", v_in);
        let c = bb.input::<i64>("value");
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
        let z = bb.output("value", 3.3f64);
        println!("z: {z:?}");
        assert!(z.is_err());
        // println!("BasicBlackboard: {bb:?}");
        // let r = bb.consumes(&TypeId::of::<i64>(), "value");
        // assert!(r.is_ok());
        // println!("value: {:?}", r.unwrap()());
        let c = bb.input::<i64>("value");
        println!("c: {c:?}");
        println!("value: {:?}", z);
    }

    use crate::{blackboard::Input, blackboard::Output, NodeType, Port};

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

        fn setup_outputs(
            &mut self,
            interface: &mut dyn BlackboardOutputInterface,
        ) -> Result<(), NodeError> {
            // let _ = direction;
            let z = interface.output::<f64>("a", 0.0)?;
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

        fn setup_inputs(
            &mut self,
            interface: &mut dyn BlackboardInputInterface,
        ) -> Result<(), NodeError> {
            self.a_input = interface.input::<f64>("a")?;
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
        let bb1 = tree.add_blackboard_boxed(
            BlackboardId(crate::Uuid::new_v4()),
            Box::new(BasicBlackboard::default()),
        )?;

        let output_ports = tree.node_ports(o1)?;
        tree.connect_port_to_blackboard(&output_ports[0], bb1)?;

        const TEST_OUTPUT_TO_MULTIPLE: bool = true;
        if TEST_OUTPUT_TO_MULTIPLE {
            let bb2 = tree.add_blackboard_boxed(
                BlackboardId(crate::Uuid::new_v4()),
                Box::new(BasicBlackboard::default()),
            )?;
            tree.connect_port_to_blackboard(&output_ports[0], bb2)?;
        }
        let input_ports = tree.node_ports(i1)?;
        tree.connect_port_to_blackboard(&input_ports[0], bb1)?;

        let res = tree.execute(root)?;
        assert_eq!(res, NodeStatus::Success);

        // get the value from the blackboard.
        let bbref = tree.blackboard_ref(bb1).unwrap();
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
