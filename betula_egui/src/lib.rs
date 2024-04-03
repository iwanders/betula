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

pub mod nodes;

mod ui;
use ui::UiConfigResponse;
pub use ui::{UiNode, UiSupport};

mod viewer;
pub use viewer::{BetulaViewer, BetulaViewerNode, ViewerNode};
