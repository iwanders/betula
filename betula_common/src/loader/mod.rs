use betula_core::prelude::*;
use betula_core::{Node, NodeId};
use serde::{Deserialize, Serialize};

// Only really used internally as an type that implements Serialize and
// Deserialize.
type NodeConfig = serde_json::Value;

#[derive(Serialize, Deserialize, Debug)]
struct TreeNode {
    id: NodeId,
    node_name: Option<String>,
    node_type: String,
    config: NodeConfig,
}

#[derive(Serialize, Deserialize, Debug)]
struct Relations {
    parent: NodeId,
    children: Vec<NodeId>,
}

#[derive(Serialize, Deserialize, Debug)]
struct TreeConfig {
    nodes: Vec<TreeNode>,
    relations: Vec<Relations>,
}
