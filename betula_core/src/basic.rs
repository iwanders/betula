/// A simple implementation of the Tree.
use crate::prelude::*;

struct BasicContext {}
impl Context for BasicContext {}

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
}

impl Tree for BasicTree {
    fn nodes(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(i, _)| NodeId(i))
            .collect()
    }

    fn add_node(&mut self, node: Box<dyn Node>) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(RefCell::new(node));
        self.children.push(vec![]);
        id
    }

    fn add_relation(&mut self, parent: NodeId, child: NodeId) {
        self.children[parent.0].push(child);
    }

    fn children(&self, id: NodeId) -> Vec<NodeId> {
        self.children[id.0].clone()
    }

    fn run(&self, id: NodeId) -> Result<Status, Error> {
        let mut n = self.nodes[id.0].borrow_mut();
        n.tick(id, self, &mut BasicContext {})
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
        let root = tree.add_node(Box::new(Fallback {}));
        let f1 = tree.add_node(Box::new(Failure {}));
        let s1 = tree.add_node(Box::new(Success {}));
        tree.add_relation(root, f1);
        tree.add_relation(root, s1);
        let res = tree.run(root);
        assert_eq!(res.ok(), Some(Status::Success));
    }
}
