use crate::{
    control::{
        BlackboardValues, CommandResult, ExecutionResult, InteractionCommand, InteractionEvent,
        TreeServer,
    },
    TreeSupport,
};
use betula_core::BetulaError;

/// Function to create the tree support in the background server thread.
pub type TreeSupportCreator = Box<dyn Fn() -> TreeSupport + Send>;

fn run_nodes(
    tree_support: &TreeSupport,
    tree: &dyn betula_core::Tree,
    roots: &[betula_core::NodeId],
) -> Result<Vec<InteractionEvent>, BetulaError> {
    let mut events = vec![];
    let mut status: Vec<betula_core::ExecutionStatus> = vec![];
    for r in roots.iter() {
        match betula_core::execute_tracked(tree, *r) {
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
            BlackboardValues::from_tree(&tree_support, tree)?,
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
        // use betula_common::control::CommandResult;
        // use betula_common::control::{InteractionEvent, BlackboardValues, ExecutionResult};

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
                            let events =
                                run_nodes(&tree_support, &mut tree, &run_settings.specific)?;
                            for e in events {
                                server.send_event(e)?;
                            }
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
                            server.send_event(InteractionEvent::CommandResult(CommandResult {
                                command: command,
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
                let events = run_nodes(&tree_support, &mut tree, &roots)?;
                for e in events {
                    server.send_event(e)?;
                }
            }
        }
    })
}
