use crate::{
    control::{
        BlackboardValues, CommandResult, ExecutionResult, InteractionCommand, InteractionEvent,
        NodeStatus, TreeServer,
    },
    TreeSupport,
};
use betula_core::{BetulaError, ExecutionStatus, NodeError, NodeId, RunContext, Tree};

use std::cell::RefCell;
struct TrackedTreeContext<'a, 'b> {
    this_node: NodeId,
    tree: &'a dyn Tree,
    status: &'b RefCell<Vec<NodeStatus>>,
}
impl RunContext for TrackedTreeContext<'_, '_> {
    fn children(&self) -> usize {
        self.tree
            .children(self.this_node)
            .expect("node must exist in tree")
            .len()
    }
    fn run(&self, index: usize) -> Result<ExecutionStatus, NodeError> {
        let ids = self.tree.children(self.this_node)?;
        let (v, all) = execute_tracked(self.tree, ids[index])?;
        let mut status = self.status.borrow_mut();
        for s in all {
            status.push(s);
        }
        v
    }
    fn reset_recursive(&self, index: usize) -> Result<(), NodeError> {
        let ids = self.tree.children(self.this_node)?;
        self.tree.reset_recursive(ids[index])
    }
}

/// Execute a node on a tree and track all node execution status.
pub fn execute_tracked(
    tree: &dyn Tree,
    id: NodeId,
) -> Result<(Result<ExecutionStatus, BetulaError>, Vec<NodeStatus>), BetulaError> {
    let mut res: RefCell<Vec<NodeStatus>> = RefCell::new(vec![]);
    let mut n = tree
        .node_ref(id)
        .ok_or_else(|| format!("node {id:?} does not exist").to_string())?
        .try_borrow_mut()?;

    let context = TrackedTreeContext {
        this_node: id,
        tree,
        status: &mut res,
    };

    let v = n.execute(&context);
    if let Err(e) = v {
        {
            let e_string = format!("{}", e);
            let mut modifyable = res.borrow_mut();
            modifyable.push(NodeStatus {
                node: id,
                status: Err(e_string),
            });
        }
        return Ok((Err(e), res.into_inner()));
    }

    let v = v.unwrap();
    {
        let mut modifyable = res.borrow_mut();
        modifyable.push(NodeStatus {
            node: id,
            status: Ok(v),
        });
    }
    Ok((Ok(v), res.into_inner()))
}

/// Function to create the tree support in the background server thread.
pub type TreeSupportCreator = Box<dyn Fn() -> TreeSupport + Send>;

fn run_nodes(
    tree_support: &TreeSupport,
    tree: &dyn betula_core::Tree,
    roots: &[betula_core::NodeId],
) -> Result<Vec<InteractionEvent>, BetulaError> {
    let mut events = vec![];
    let mut status: Vec<NodeStatus> = vec![];
    for r in roots.iter() {
        match execute_tracked(tree, *r) {
            Ok((_this_node, mut all_nodes)) => {
                status.extend(&mut all_nodes.drain(..));
            }
            Err(_e) => {
                // println!("Failed running {r:?}: {e:?}");
            }
        }
    }

    if !status.is_empty() {
        events.push(InteractionEvent::ExecutionResult(ExecutionResult {
            node_status: status,
        }));
    }

    // Dump all blackboard values to the frontend for now.
    if !roots.is_empty() {
        events.push(InteractionEvent::BlackboardValues(
            BlackboardValues::from_tree(tree_support, tree)?,
        ));
    }
    Ok(events)
}

/// Function to run a Tree and TreeServer in the background.
pub fn create_server_thread<T: betula_core::Tree, B: betula_core::Blackboard + 'static>(
    tree_support: TreeSupportCreator,
    server: impl TreeServer + std::marker::Send + 'static,
) -> std::thread::JoinHandle<Result<(), BetulaError>> {
    std::thread::spawn(move || -> Result<(), betula_core::BetulaError> {
        let mut tree = T::new();
        let tree_support = tree_support();

        let mut run_roots: bool = false;
        let mut sleep_interval = std::time::Duration::from_millis(10);
        loop {
            std::thread::sleep(sleep_interval);

            loop {
                let received = server.get_command()?;
                if let Some(command) = received {
                    println!("    Executing {command:?}");
                    if let InteractionCommand::RunSettings(run_settings) = &command {
                        if let Some(new_value) = run_settings.roots {
                            // println!("Setting run roots to: {new_value}");
                            run_roots = new_value;
                        }
                        if let Some(new_duration) = run_settings.interval {
                            sleep_interval = new_duration;
                        }
                        if !run_settings.specific.is_empty() {
                            let events = run_nodes(&tree_support, &tree, &run_settings.specific)?;
                            for e in events {
                                server.send_event(e)?;
                            }
                            continue; // continue to prevent the execute section from also running.
                        }
                    }
                    let r = command.execute(&tree_support, &mut tree);
                    match r {
                        Ok(v) => {
                            for event in v {
                                server.send_event(event)?;
                            }
                        }
                        Err(e) => {
                            println!("failed to execute: {e:?}");
                            server.send_event(InteractionEvent::CommandResult(CommandResult {
                                command,
                                error: Some(format!("{e:?}")),
                            }))?;
                        }
                    }
                } else {
                    break;
                }
            }

            if run_roots {
                let roots = tree.roots();
                let events = run_nodes(&tree_support, &tree, &roots)?;
                for e in events {
                    server.send_event(e)?;
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use betula_core::basic::BasicTree;
    use betula_std::nodes::*;
    use uuid::Uuid;

    #[test]
    fn fallback_tracked() -> Result<(), NodeError> {
        // use crate::TrackedTreeExecution;
        let mut tree: Box<dyn Tree> = Box::new(BasicTree::new());
        let root_id = NodeId(Uuid::new_v4());
        let root = tree.add_node_boxed(root_id, Box::new(SelectorNode::default()))?;
        let f1_id = NodeId(Uuid::new_v4());
        let f1 = tree.add_node_boxed(f1_id, Box::new(FailureNode {}))?;
        let s1_id = NodeId(Uuid::new_v4());
        let s1 = tree.add_node_boxed(s1_id, Box::new(SuccessNode {}))?;
        tree.set_children(root, &vec![f1, s1])?;
        let (this_node, all_nodes) = execute_tracked(&*tree, root)?;
        println!("All nodes: {all_nodes:#?}");
        assert_eq!(this_node.ok(), Some(ExecutionStatus::Success));
        assert_eq!(all_nodes.len(), 3);
        assert_eq!(all_nodes[0].node, f1_id);
        assert_eq!(all_nodes[0].status, Ok(ExecutionStatus::Failure));
        assert_eq!(all_nodes[1].node, s1_id);
        assert_eq!(all_nodes[1].status, Ok(ExecutionStatus::Success));
        assert_eq!(all_nodes[2].node, root_id);
        assert_eq!(all_nodes[2].status, Ok(ExecutionStatus::Success));
        Ok(())
    }
}
