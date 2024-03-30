use betula_core::prelude::*;
use betula_core::{BlackboardId, NodeId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
struct ViewerNode {
    id: NodeId,
}

#[derive(Clone, Serialize, Deserialize)]
struct ViewerBlackboard {
    id: BlackboardId,
}

#[derive(Clone, Serialize, Deserialize)]
enum BetulaViewerNode {
    Node(ViewerNode),
    Blackboard(ViewerBlackboard),
}

pub struct BetulaViewer {}
