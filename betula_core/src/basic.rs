/// A simple implementation of the Tree.
use crate::prelude::*;

struct TreeContext<'a> {
    this_node: usize,
    tree: &'a BasicTree,
}
impl RunContext for TreeContext<'_> {
    fn children(&self) -> usize {
        self.tree.children(NodeId(self.this_node)).len()
    }
    fn run(&self, index: usize) -> Result<Status, Error> {
        let ids = self.tree.children(NodeId(self.this_node));
        self.tree.run(ids[index])
    }
}

/// Node Id that's used to refer to nodes in a context.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct NodeId(pub usize);

use std::cell::RefCell;
#[derive(Debug)]
pub struct BasicTree {
    nodes: Vec<RefCell<Box<dyn Node>>>,
    children: Vec<Vec<NodeId>>,
}

impl BasicTree {
    pub fn new() -> Self {
        BasicTree {
            nodes: vec![],
            children: vec![],
        }
    }
    pub fn get_node(&self, id: NodeId) -> &RefCell<Box<dyn Node>> {
        &self.nodes[id.0]
    }

    pub fn nodes(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(i, _)| NodeId(i))
            .collect()
    }

    pub fn add_node(&mut self, node: Box<dyn Node>) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(RefCell::new(node));
        self.children.push(vec![]);
        id
    }

    pub fn add_relation(&mut self, parent: NodeId, child: NodeId) {
        self.children[parent.0].push(child);
    }

    pub fn children(&self, id: NodeId) -> Vec<NodeId> {
        self.children[id.0].clone()
    }

    pub fn run(&self, id: NodeId) -> Result<Status, Error> {
        let mut n = self.nodes[id.0].try_borrow_mut()?;
        let mut context = TreeContext {
            this_node: id.0,
            tree: &self,
        };

        n.tick(&mut context)
    }
}

use std::collections::HashMap;
use std::rc::Rc;
// use std::cell::RefCell;
use std::any::Any;

// use crate::BlackboardValue;
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
    fn sequence_fail() {
        let mut tree = BasicTree::new();
        let root = tree.add_node(Box::new(Sequence {}));
        let f1 = tree.add_node(Box::new(Failure {}));
        tree.add_relation(root, f1);
        let res = tree.run(root);
        assert_eq!(res.ok(), Some(Status::Failure));
    }

    #[test]
    fn fallback_success() {
        let mut tree = BasicTree::new();
        let root = tree.add_node(Box::new(Selector {}));
        let f1 = tree.add_node(Box::new(Failure {}));
        let s1 = tree.add_node(Box::new(Success {}));
        tree.add_relation(root, f1);
        tree.add_relation(root, s1);
        let res = tree.run(root);
        assert_eq!(res.ok(), Some(Status::Success));
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
