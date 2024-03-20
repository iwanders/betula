/// A simple implementation of the Tree.
use crate::prelude::*;
use std::collections::HashMap;

use crate::{Error, Node, NodeId, Status};

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
    fn run(&self, index: usize) -> Result<Status, Error> {
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
    fn ids(&self) -> Vec<NodeId> {
        self.nodes.keys().copied().collect()
    }
    fn node_mut(&mut self, id: NodeId) -> Option<&mut dyn Node> {
        let m = self.nodes.get_mut(&id)?;
        Some(&mut **m.node.get_mut())
    }
    fn remove_node(&mut self, id: NodeId) -> Result<(), Error> {
        for (_k, v) in self.nodes.iter_mut() {
            v.children.retain(|&x| x != id);
        }
        self.nodes
            .remove(&id)
            .ok_or_else(|| format!("id {id:?} is not present").into())
            .map(|_| ())
    }

    fn add_node_boxed(&mut self, id: NodeId, node: Box<dyn Node>) -> Result<NodeId, Error> {
        self.nodes.insert(
            id,
            BasicTreeNode {
                node: node.into(),
                children: vec![],
            },
        );
        Ok(id)
    }

    fn children(&self, id: NodeId) -> Result<Vec<NodeId>, Error> {
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
    ) -> Result<(), Error> {
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
    fn remove_relation(&mut self, parent: NodeId, position: usize) -> Result<(), Error> {
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

    fn execute(&self, id: NodeId) -> Result<Status, Error> {
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
    fn setup(&mut self, id: NodeId, ctx: &mut dyn Interface) -> Result<(), Error> {
        let mut n = self
            .nodes
            .get(&id)
            .ok_or_else(|| format!("node {id:?} does not exist").to_string())?
            .node
            .try_borrow_mut()?;
        n.setup(ctx)
    }
}

use std::any::Any;
use std::rc::Rc;

use crate::blackboard::{Interface, Read, Value, ValueCreator, Write};

use std::any::TypeId;
#[derive(Default, Debug)]
pub struct BasicBlackboard {
    values: HashMap<String, (TypeId, Rc<RefCell<Value>>)>,
}
use crate::as_any::AsAny;
impl Interface for BasicBlackboard {
    fn writer(&mut self, id: TypeId, key: &str, default: ValueCreator) -> Result<Write, Error> {
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

    fn reader(&mut self, id: &TypeId, key: &str) -> Result<Read, Error> {
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
    fn sequence_fail() -> Result<(), Error> {
        let mut tree = BasicTree::new();
        let root = tree.add_node_boxed(NodeId(crate::Uuid::new_v4()), Box::new(Sequence {}))?;
        let f1 = tree.add_node_boxed(NodeId(crate::Uuid::new_v4()), Box::new(Failure {}))?;
        tree.add_relation(root, 0, f1)?;
        let res = tree.execute(root)?;
        assert_eq!(res, Status::Failure);
        Ok(())
    }

    #[test]
    fn fallback_success() -> Result<(), Error> {
        let mut tree = BasicTree::new();
        let root = tree.add_node_boxed(NodeId(crate::Uuid::new_v4()), Box::new(Selector {}))?;
        let f1 = tree.add_node_boxed(NodeId(crate::Uuid::new_v4()), Box::new(Failure {}))?;
        let s1 = tree.add_node_boxed(NodeId(crate::Uuid::new_v4()), Box::new(Success {}))?;
        tree.add_relation(root, 0, f1)?;
        tree.add_relation(root, 1, s1)?;
        let res = tree.execute(root)?;
        assert_eq!(res, Status::Success);
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
