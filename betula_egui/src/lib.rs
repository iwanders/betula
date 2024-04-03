/*! A viewer for Betula Behaviour trees.
*/

/*
Goals:
    - We really want the UI to be able to show blackboard values.
    - As well as possibly inner state of nodes.
    - Need to be able to modify the node's configuration with a nice ui.
    - Want the UI to be able to modify the tree, without losing state in the tree.

Thoughts on interaction:
    - We don't want the UI to own the tree.
    - Tree should be able to run quickly in the background.
    - We don't really want the UI to directly access the tree with a mutex.
      Because that'll likely result in contention, as well as a slower running
      tree if we have a UI running.

Future:
    - Tree needs to be able to run in a different process, where we hook
      up the viewer.


Notes:
    A node is up to date if the remote (server) data matches the client.
    A node is dirty if the snarl state needs updating.

*/

/*
To gui:
    - Node State Change
    - Blackboard Value


    - Results from anything sent to the tree.

To tree:
    - Node / blackboard addition
    - Port / Relation changes.
    - Node configuration set.


For nodes:
    Outputs:
        Ports are first.
        Children are second.
    Inputs:
        Parent is 0.
        Ports are second.
*/

// use betula_core::prelude::*;
// , NodeType
use betula_core::{
    BetulaError, BlackboardId, Node, NodeId as BetulaNodeId, NodeType, PortDirection,
};
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use uuid::Uuid;

pub mod nodes;

use egui::{Color32, Ui};

mod ui;
use ui::UiConfigResponse;
pub use ui::{UiNode, UiSupport};

mod viewer;
pub use viewer::{BetulaViewer, BetulaViewerNode, ViewerNode};

#[cfg(test)]
mod test {
    use super::*;
    use betula_common::control::InProcessControlServer;

    fn make_server_check(
        server: InProcessControlServer,
    ) -> std::thread::JoinHandle<Result<(), betula_core::BetulaError>> {
        use betula_core::basic::BasicTree;
        std::thread::spawn(move || -> Result<(), betula_core::BetulaError> {
            use betula_common::control::TreeServer;
            // use betula_core::Tree;

            use betula_common::control::CommandResult;
            use betula_common::control::InteractionEvent;

            let mut tree = BasicTree::new();
            let mut tree_support = TreeSupport::new();
            tree_support.add_node_default::<betula_core::nodes::SequenceNode>();
            tree_support.add_node_default::<betula_core::nodes::SelectorNode>();
            tree_support.add_node_default::<betula_core::nodes::FailureNode>();
            tree_support.add_node_default::<betula_core::nodes::SuccessNode>();
            tree_support.add_node_default_with_config::<betula_common::nodes::DelayNode, betula_common::nodes::DelayNodeConfig>(
                );
            tree_support.add_node_default::<betula_common::nodes::TimeNode>();
            tree_support.add_node_default::<betula_common::nodes::DelayNode>();
            tree_support.add_value_default::<f64>();

            loop {
                let received = server.get_command();
                if received.is_err() {
                    break;
                }
                let received = received.unwrap();

                if let Some(command) = received {
                    println!("    Executing {command:?}");
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
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
            Ok(())
        })
    }

    #[test]
    fn test_connection() -> Result<(), BetulaError> {
        use betula_common::control::InProcessControl;
        let (server, client) = InProcessControl::new();
        use uuid::uuid;
        let delay1 = BetulaNodeId(uuid!("00000000-0000-0000-0000-ffff00000001"));
        let delay2 = BetulaNodeId(uuid!("00000000-0000-0000-0000-ffff00000002"));
        let delay3 = BetulaNodeId(uuid!("00000000-0000-0000-0000-ffff00000003"));

        let server_thing = make_server_check(server);

        let mut snarl = Snarl::<BetulaViewerNode>::new();
        {
            let mut viewer = BetulaViewer::new(Box::new(client));
            viewer
                .client()
                .send_command(InteractionCommand::tree_call(move |tree| {
                    assert!(tree.nodes().len() == 0);
                    Ok(())
                }))?;
            viewer.ui_create_node(
                delay1,
                egui::pos2(0.0, 0.0),
                betula_common::nodes::DelayNode::static_type(),
                &mut snarl,
            );
            viewer.ui_create_node(
                delay2,
                egui::pos2(0.0, 0.0),
                betula_common::nodes::DelayNode::static_type(),
                &mut snarl,
            );
            viewer.ui_create_node(
                delay3,
                egui::pos2(0.0, 0.0),
                betula_common::nodes::DelayNode::static_type(),
                &mut snarl,
            );
            std::thread::sleep(std::time::Duration::from_millis(50));
            // Verify that the tree now has 3 nodes.
            viewer
                .client()
                .send_command(InteractionCommand::tree_call(move |tree| {
                    assert!(tree.nodes().len() == 3);
                    Ok(())
                }))?;
            // Next, setup relations.
            viewer.service(&mut snarl)?;
            viewer.connect_relation(delay1, delay2, 0, &mut snarl)?;
            std::thread::sleep(std::time::Duration::from_millis(50));
            viewer.service(&mut snarl)?;
            viewer.connect_relation(delay1, delay3, 1, &mut snarl)?;
            std::thread::sleep(std::time::Duration::from_millis(50));
            viewer.service(&mut snarl)?;
            std::thread::sleep(std::time::Duration::from_millis(50));
            // Verify the children of delay 1.
            viewer
                .client()
                .send_command(InteractionCommand::tree_call(move |tree| {
                    println!("testing");
                    assert!(tree.nodes().len() == 3);
                    assert!(tree.children(delay1)? == vec![delay2, delay3]);
                    Ok(())
                }))?;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(server_thing.join().is_ok());
        Ok(())
    }
}
