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
    BetulaError, BlackboardId, Node, NodeConfig, NodeId as BetulaNodeId, NodeType, Port,
};
use serde::{Deserialize, Serialize};

use uuid::Uuid;

pub mod nodes;

use egui_snarl::{
    ui::{PinInfo, SnarlViewer},
    InPin, NodeId as SnarlNodeId, OutPin, Snarl,
};

use betula_common::control::TreeClient;

#[derive(Clone, Serialize, Deserialize)]
pub struct ViewerNode {
    id: BetulaNodeId,

    #[serde(skip)]
    node_type: Option<NodeType>,

    #[serde(skip)]
    node_config: Option<Box<dyn NodeConfig>>,
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

pub trait NodeUi {
    fn name(&self) -> String;
    fn child_constraints(&self, _node: &mut ViewerNode) -> std::ops::Range<usize> {
        0..usize::MAX
    }
    fn ports(&self, _node: &ViewerNode) -> Vec<Port> {
        vec![]
    }
    fn ui_title(&self, _node: &ViewerNode) -> String {
        self.name()
    }
    fn has_config(&self, _node: &ViewerNode) -> bool {
        false
    }
    fn ui_config(&self, _node: &mut ViewerNode, _ui: &mut Ui, _scale: f32) {}
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

pub struct BetulaViewer {
    // Some ui support... for stuff like configs.
    client: Box<dyn TreeClient>,
    ui_support: UiSupport,

    node_map: HashMap<BetulaNodeId, SnarlNodeId>,
}
impl BetulaViewer {
    pub fn new(client: Box<dyn TreeClient>) -> Self {
        let mut ui_support = UiSupport::new();
        ui_support.add_node_default::<betula_core::nodes::SequenceNode>();
        ui_support.add_node_default::<betula_core::nodes::SelectorNode>();
        ui_support.add_node_default::<betula_core::nodes::FailureNode>();
        ui_support.add_node_default::<betula_core::nodes::SuccessNode>();
        ui_support.add_node_default::<betula_common::nodes::DelayNode>();
        ui_support.add_node_default::<betula_common::nodes::TimeNode>();
        BetulaViewer {
            client,
            ui_support,
            node_map: Default::default(),
        }
    }

    fn get_node_mut<'a>(
        &self,
        node_id: BetulaNodeId,
        snarl: &'a mut Snarl<BetulaViewerNode>,
    ) -> Result<&'a mut ViewerNode, BetulaError> {
        if let Some(snarl_id) = self.node_map.get(&node_id) {
            let node = snarl.get_node_mut(*snarl_id);
            if let Some(viewer_node) = node {
                if let BetulaViewerNode::Node(viewer_node) = viewer_node {
                    return Ok(viewer_node);
                } else {
                    Err(format!("snarl node {node_id:?} is no node").into())
                }
            } else {
                Err(format!("snarl node {node_id:?} cannot be found").into())
            }
        } else {
            Err(format!("node {node_id:?} cannot be found").into())
        }
    }

    pub fn service(&mut self, snarl: &mut Snarl<BetulaViewerNode>) -> Result<(), BetulaError> {
        let event = self.client.get_event()?;
        use betula_common::control::InteractionEvent::NodeInformation;

        if let Some(event) = event {
            println!("event {event:?}");
            match event {
                NodeInformation(v) => {
                    let viewer_node = self.get_node_mut(v.id, snarl)?;
                    viewer_node.node_type = Some(v.node_type);
                    Ok(())
                }
                unknown => Err(format!("Unhandled event {unknown:?}").into()),
            }
        } else {
            Ok(())
        }
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
                    use betula_common::control::InteractionCommand;
                    let id = BetulaNodeId(Uuid::new_v4());
                    let cmd = InteractionCommand::add_node(id, node_type);
                    if let Ok(_) = self.client.send_command(cmd) {
                        let snarl_id = snarl.insert_node(
                            pos,
                            BetulaViewerNode::Node(ViewerNode {
                                id,
                                node_type: None,
                                node_config: None,
                            }),
                        );
                        self.node_map.insert(id, snarl_id);
                    }
                    ui.close_menu();
                }
            }
        }
    }

    fn node_menu(
        &mut self,
        node: SnarlNodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<BetulaViewerNode>,
    ) {
        ui.label("Node menu");
        if ui.button("Remove").clicked() {
            snarl.remove_node(node);
            ui.close_menu();
        }
    }

    fn has_footer(&mut self, node: &BetulaViewerNode) -> bool {
        false
    }
}
