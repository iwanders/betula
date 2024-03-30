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
use betula_core::{
    BetulaError, BlackboardId, Node, NodeId, NodeStatus, NodeType, Port, RunContext,
};
use serde::{Deserialize, Serialize};

use uuid::Uuid;

pub mod nodes;

use egui_snarl::{
    ui::{PinInfo, SnarlStyle, SnarlViewer},
    InPin, NodeId as SnarlNodeId, OutPin, Snarl,
};

use betula_common::{control::TreeClient, TreeSupport};

#[derive(Clone, Serialize, Deserialize)]
pub struct ViewerNode {
    id: NodeId,

    #[serde(skip)]
    node_type: Option<NodeType>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ViewerBlackboard {
    id: BlackboardId,
    // Full, or just a single?
}

#[derive(Clone, Serialize, Deserialize)]
pub enum BetulaViewerNode {
    Node(ViewerNode),
    Blackboard(ViewerBlackboard),
}

pub struct BetulaViewer {
    // Some ui support... for stuff like configs.
    client: Box<dyn TreeClient>,
    ui_support: UiSupport,
}

pub trait NodeUi {
    fn name(&self) -> String;
    fn child_constraints(&self, node: &mut ViewerNode) -> std::ops::Range<usize> {
        0..usize::MAX
    }
    fn ports(&self, node: &ViewerNode) -> Vec<Port> {
        vec![]
    }
    fn ui_title(&self, node: &ViewerNode) -> String {
        self.name()
    }
    fn has_config(&self, node: &ViewerNode) -> bool {
        false
    }
    fn ui_config(&self, node: &ViewerNode, ui: &mut Ui, scale: f32) {}
}

use std::collections::HashMap;
struct UiSupport {
    node_support: HashMap<NodeType, Box<dyn NodeUi>>,
}
impl UiSupport {
    pub fn new() -> Self {
        Self {
            node_support: Default::default(),
        }
    }
    pub fn add_node_default<T: Node + NodeUi + Default + 'static>(&mut self) {
        self.node_support
            .insert(T::static_type(), Box::new(T::default()));
    }
    pub fn node_types(&self) -> Vec<NodeType> {
        self.node_support.keys().cloned().collect()
    }
    pub fn get_node_support(&self, node_type: &NodeType) -> Option<&dyn NodeUi> {
        self.node_support.get(node_type).map(|v| &**v)
    }
}

impl BetulaViewer {
    pub fn new(client: Box<dyn TreeClient>) -> Self {
        let mut ui_support = UiSupport::new();
        ui_support.add_node_default::<betula_core::nodes::SequenceNode>();
        ui_support.add_node_default::<betula_core::nodes::SelectorNode>();
        ui_support.add_node_default::<betula_core::nodes::FailureNode>();
        ui_support.add_node_default::<betula_core::nodes::SuccessNode>();
        BetulaViewer { client, ui_support }
    }
}

use egui::{Color32, Ui};

impl SnarlViewer<BetulaViewerNode> for BetulaViewer {
    fn title(&mut self, node: &BetulaViewerNode) -> std::string::String {
        match node {
            BetulaViewerNode::Node(node) => {
                // Grab the type support for this node.
                if let Some(node_type) = &node.node_type {
                    if let Some(support) = self.ui_support.node_support.get(node_type) {
                        support.ui_title(node)
                    } else {
                        format!("{node_type:?}")
                    }
                } else {
                    "Pending...".to_owned()
                }
            }
            _ => todo!(),
        }
    }
    fn outputs(&mut self, _: &BetulaViewerNode) -> usize {
        0
    }
    fn inputs(&mut self, _: &BetulaViewerNode) -> usize {
        0
    }
    fn show_input(
        &mut self,
        _: &InPin,
        _: &mut Ui,
        _: f32,
        _: &mut Snarl<BetulaViewerNode>,
    ) -> PinInfo {
        todo!()
    }
    fn show_output(
        &mut self,
        _: &OutPin,
        _: &mut Ui,
        _: f32,
        _: &mut Snarl<BetulaViewerNode>,
    ) -> PinInfo {
        todo!()
    }
    fn input_color(
        &mut self,
        _: &InPin,
        _: &egui::style::Style,
        _: &mut Snarl<BetulaViewerNode>,
    ) -> Color32 {
        todo!()
    }
    fn output_color(
        &mut self,
        _: &OutPin,
        _: &egui::style::Style,
        _: &mut Snarl<BetulaViewerNode>,
    ) -> Color32 {
        todo!()
    }

    fn graph_menu(
        &mut self,
        pos: egui::Pos2,
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<BetulaViewerNode>,
    ) {
        ui.label("Add node:");
        for node_type in self.ui_support.node_types() {
            let support = self.ui_support.get_node_support(&node_type);
            if let Some(support) = support {
                if ui.button(support.name()).clicked() {
                    use betula_common::control::AddNodeCommand;
                    let id = NodeId(Uuid::new_v4());
                    let add_cmd = AddNodeCommand { id, node_type };
                    if let Ok(_) = self
                        .client
                        .send_command(betula_common::control::InteractionCommand::AddNode(add_cmd))
                    {
                        snarl.insert_node(
                            pos,
                            BetulaViewerNode::Node(ViewerNode {
                                id,
                                node_type: None,
                            }),
                        );
                    }
                    ui.close_menu();
                }
            }
        }
    }
}
