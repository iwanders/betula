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

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
// use std::cell::RefCell;

use std::any::TypeId;
pub struct BasicBlackboard {
    values: HashMap<(TypeId, String), Rc<RefCell<Box<dyn std::any::Any>>>>,
}

struct BasicProvider<T> {
    v: Rc<Cell<Box<T>>>,
}
impl<T> crate::Provider for BasicProvider<T> {
    type ProviderItem = T;
    fn set(&self, v: Self::ProviderItem) -> Result<Self::ProviderItem, Error> {
        // let z = self.v.get();

        Err("dlkjsfls".into())
    }
}

impl crate::BlackboardContext for BasicBlackboard {
    fn provides(
        &mut self,
        id: &TypeId,
        key: &str,
        default: crate::BlackboardValueCreator,
    ) -> Box<dyn std::any::Any> {
        let rc = self
            .values
            .entry((*id, key.to_string()))
            .or_insert_with(|| Rc::new(RefCell::new(default())))
            .clone();
        Box::new(0)
    }
    fn consumes(&mut self, id: &TypeId, key: &str) -> Box<dyn std::any::Any> {
        // let cloned_rc = self.values.get(&(*id, key.to_string())).clone();
        // Box::new(cloned_rc)
        Box::new(0)
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
}
