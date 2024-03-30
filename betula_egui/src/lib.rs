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

*/

// use betula_core::prelude::*;
// , NodeType
use betula_core::{BlackboardId, NodeId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
struct ViewerNode {
    id: NodeId,
}

#[derive(Clone, Serialize, Deserialize)]
struct ViewerBlackboard {
    id: BlackboardId,
    // Full, or just a single?
}

#[derive(Clone, Serialize, Deserialize)]
enum BetulaViewerNode {
    Node(ViewerNode),
    Blackboard(ViewerBlackboard),
}

pub struct GuiSupport {}

pub struct BetulaViewer {
    // Some ui support... for stuff like configs.
}
