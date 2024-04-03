use betula_common::TreeSupport;
use egui::{Color32, Ui};
use std::collections::HashMap;

use betula_core::{
    BetulaError, BlackboardId, Node, NodeId as BetulaNodeId, NodeType, PortDirection,
};

#[derive(PartialEq, Clone, Copy, Hash, Debug, Eq)]
pub enum UiConfigResponse {
    /// Config has changed, needs to be sent to the server.
    UnChanged,
    /// Config is unchanged, no action necessary.
    Changed,
}

/// Trait for nodes in the ui.
///
/// It will never be executed, but sharing functionality from Node is
/// useful as it allows reusing the get_config and set_config methods as
/// well as the ports function.
pub trait UiNode: Node {
    fn ui_title(&self) -> String {
        self.node_type().0.clone()
    }

    fn ui_child_range(&self) -> std::ops::Range<usize> {
        0..usize::MAX
    }

    fn ui_config(&mut self, _ui: &mut Ui, _scale: f32) -> UiConfigResponse {
        UiConfigResponse::UnChanged
    }

    fn ui_output_port_count(&self) -> usize {
        self.ports()
            .unwrap_or(vec![])
            .iter()
            .filter(|p| p.direction() == PortDirection::Output)
            .count()
    }
    fn ui_input_port_count(&self) -> usize {
        self.ports()
            .unwrap_or(vec![])
            .iter()
            .filter(|p| p.direction() == PortDirection::Input)
            .count()
    }
}

type UiNodeFactory = Box<dyn Fn() -> Box<dyn UiNode>>;
struct UiNodeSupport {
    // node_type: NodeType,
    display_name: String,
    node_factory: UiNodeFactory,
}

pub struct UiSupport {
    ui: HashMap<NodeType, UiNodeSupport>,
    tree: TreeSupport,
}
impl UiSupport {
    pub fn new() -> Self {
        Self {
            ui: Default::default(),
            tree: Default::default(),
        }
    }

    pub fn tree_support(&self) -> &TreeSupport {
        &self.tree
    }

    pub fn add_node_default<T: Node + UiNode + Default + 'static>(&mut self) {
        self.tree.add_node_default::<T>();
        let ui_support = UiNodeSupport {
            display_name: T::static_type().0.clone(),
            node_factory: Box::new(|| Box::new(T::default())),
        };
        self.ui.insert(T::static_type(), ui_support);
    }

    pub fn add_node_default_with_config<
        N: Node + UiNode + Default + 'static,
        C: betula_common::type_support::DefaultConfigRequirements,
    >(
        &mut self,
    ) {
        self.tree.add_node_default_with_config::<N, C>();
        self.add_node_default::<N>();
    }

    pub fn node_types(&self) -> Vec<NodeType> {
        self.ui.keys().cloned().collect()
    }

    pub fn display_name(&self, node_type: &NodeType) -> String {
        if let Some(node_support) = self.ui.get(node_type) {
            node_support.display_name.clone()
        } else {
            "Unknown Node".into()
        }
    }

    pub fn create_ui_node(&self, node_type: &NodeType) -> Result<Box<dyn UiNode>, BetulaError> {
        if let Some(node_support) = self.ui.get(node_type) {
            Ok((node_support.node_factory)())
        } else {
            Err("no ui node support for {node_type:?}".into())
        }
    }
}
