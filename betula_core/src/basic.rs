/// A simple implementation of the Tree.
use crate::prelude::*;
use std::collections::HashMap;

use crate::{BetulaError, Node, NodeError, NodeId, NodeStatus};

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
#[derive(Debug)]
struct BasicTreeNode {
    node: RefCell<Box<dyn Node>>,
    children: Vec<NodeId>,
}
#[derive(Debug, Default)]
pub struct BasicTree {
    nodes: HashMap<NodeId, BasicTreeNode>,
}

impl BasicTree {
    pub fn new() -> Self {
        BasicTree {
            nodes: HashMap::default(),
        }
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
    fn remove_node(&mut self, id: NodeId) -> Result<(), BetulaError> {
        for (_k, v) in self.nodes.iter_mut() {
            v.children.retain(|&x| x != id);
        }
        self.nodes
            .remove(&id)
            .ok_or_else(|| format!("id {id:?} is not present").into())
            .map(|_| ())
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

    fn add_relation(
        &mut self,
        parent: NodeId,
        position: usize,
        child: NodeId,
    ) -> Result<(), BetulaError> {
        let n = self
            .nodes
            .get_mut(&parent)
            .ok_or_else(|| format!("node {parent:?} is not present").to_string())?;
        if position > n.children.len() {
            // insert would panic, lets raise an error
            return Err(format!("position {position} is too large").into());
        }
        n.children.insert(position, child);
        Ok(())
    }
    fn remove_relation(&mut self, parent: NodeId, position: usize) -> Result<(), BetulaError> {
        let n = self
            .nodes
            .get_mut(&parent)
            .ok_or_else(|| format!("node {parent:?} is not present").to_string())?;
        if position >= n.children.len() {
            // insert would panic, lets raise an error
            return Err(format!("position {position} is too large").into());
        }
        n.children.remove(position);
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

    /// Call setup on a particular node.
    fn port_setup(
        &mut self,
        id: NodeId,
        port: &crate::DirectionalPort,
        interface: &mut dyn BlackboardInterface,
    ) -> Result<(), NodeError> {
        let mut n = self
            .nodes
            .get(&id)
            .ok_or_else(|| format!("node {id:?} does not exist").to_string())?
            .node
            .try_borrow_mut()?;
        n.port_setup(port, interface)
    }
}

use std::any::Any;
use std::rc::Rc;

use crate::blackboard::{BlackboardInterface, Read, Value, ValueCreator, Write};

use std::any::TypeId;
#[derive(Default, Debug)]
pub struct BasicBlackboard {
    values: HashMap<String, (TypeId, Rc<RefCell<Value>>)>,
}
use crate::as_any::AsAny;
impl BlackboardInterface for BasicBlackboard {
    fn writer(&mut self, id: TypeId, key: &str, default: ValueCreator) -> Result<Write, NodeError> {
        let (typeid, rc) = self
            .values
            .entry(key.to_string())
            .or_insert_with(|| (id, Rc::new(RefCell::new(default()))))
            .clone();
        let temp_rc = rc.clone();
        let current_type = {
            let z = temp_rc
                .try_borrow_mut()
                .or_else(|_| Err(format!("{key} was already borrowed")))?;
            (**z).type_name().to_string()
        };
        let owned_key = key.to_string();
        if typeid != id {
            Err(format!(
                "new writer for '{key}', has wrong type: already got {}",
                current_type
            )
            .into())
        } else {
            Ok(Box::new(move |v: Value| {
                let mut locked = rc.try_borrow_mut()?;
                if (**locked).type_id() != (*v).type_id() {
                    Err(format!(
                        "assignment for '{owned_key}' is incorrect type {} expected {}",
                        (**locked).type_name(),
                        (*v).type_name()
                    )
                    .into())
                } else {
                    *locked = v;
                    Ok(())
                }
            }))
        }
    }

    fn reader(&mut self, id: &TypeId, key: &str) -> Result<Read, NodeError> {
        let (typeid, rc) = self
            .values
            .get(key)
            .ok_or_else(|| format!("key '{key}' not found"))?;
        let v = rc.clone();
        if typeid != id {
            Err(format!(
                "new reader for '{key}' mismatches type: already got {}",
                rc.type_name()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::*;

    #[test]
    fn sequence_fail() -> Result<(), NodeError> {
        let mut tree = BasicTree::new();
        let root = tree.add_node_boxed(NodeId(crate::Uuid::new_v4()), Box::new(SequenceNode {}))?;
        let f1 = tree.add_node_boxed(NodeId(crate::Uuid::new_v4()), Box::new(FailureNode {}))?;
        tree.add_relation(root, 0, f1)?;
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
        tree.add_relation(root, 0, f1)?;
        tree.add_relation(root, 1, s1)?;
        let res = tree.execute(root)?;
        assert_eq!(res, NodeStatus::Success);
        Ok(())
    }

    #[test]
    fn blackboard_provider() {
        let mut bb = BasicBlackboard::default();

        // let mut w = crate::BlackboardContext::new(&mut bb);
        let v_in = 3i64;
        let p = bb.provides("value", v_in);
        let c = bb.consumes::<i64>("value");
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
        let z = bb.provides("value", 3.3f64);
        println!("z: {z:?}");
        assert!(z.is_err());
        // println!("BasicBlackboard: {bb:?}");
        // let r = bb.consumes(&TypeId::of::<i64>(), "value");
        // assert!(r.is_ok());
        // println!("value: {:?}", r.unwrap()());
        let c = bb.consumes::<i64>("value");
        println!("c: {c:?}");
        println!("value: {:?}", z);
    }
}
