use betula_common::TreeSupport;
use egui::Ui;
use std::collections::HashMap;

use betula_core::{BetulaError, Node, NodeType, Port, PortDirection};

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
    /// The title for this ui node.
    fn ui_title(&self) -> String {
        self.node_type().0.clone()
    }

    /// The range of children this node has.
    fn ui_child_range(&self) -> std::ops::Range<usize> {
        0..usize::MAX
    }

    /// Function to render the ui, responds whether changes were made.
    fn ui_config(&mut self, _ui: &mut Ui, _scale: f32) -> UiConfigResponse {
        UiConfigResponse::UnChanged
    }

    /// The number of output ports this node has in the ui.
    fn ui_output_port_count(&self) -> usize {
        self.ports()
            .unwrap_or(vec![])
            .iter()
            .filter(|p| p.direction() == PortDirection::Output)
            .count()
    }

    /// The number of input ports this node has in the ui.
    fn ui_input_port_count(&self) -> usize {
        self.ports()
            .unwrap_or(vec![])
            .iter()
            .filter(|p| p.direction() == PortDirection::Input)
            .count()
    }

    /// The port to show at this input number.
    fn ui_input_port(&self, input: usize) -> Option<Port> {
        self.ports()
            .unwrap_or(vec![])
            .iter()
            .filter(|p| p.direction() == PortDirection::Input)
            .nth(input)
            .cloned()
    }
}

type UiNodeFactory = Box<dyn Fn() -> Box<dyn UiNode>>;
pub struct UiNodeSupport {
    pub display_name: String,
    pub node_factory: UiNodeFactory,
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

    pub fn ui_support(&self, node_type: &NodeType) -> Option<&UiNodeSupport> {
        self.ui.get(node_type)
    }

    pub fn add_node_default<
        T: UiNode + betula_common::type_support::DefaultNodeFactoryRequirements,
    >(
        &mut self,
    ) {
        self.tree.add_node_default::<T>();
        let ui_support = UiNodeSupport {
            display_name: T::static_type().0.clone(),
            node_factory: Box::new(|| Box::new(T::default())),
        };
        self.ui.insert(T::static_type(), ui_support);
    }

    pub fn add_node_default_with_config<
        N: UiNode + betula_common::type_support::DefaultNodeFactoryRequirements,
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
        if let Some(node_support) = self.ui_support(node_type) {
            node_support.display_name.clone()
        } else {
            "Unknown Node".into()
        }
    }

    pub fn create_ui_node(&self, node_type: &NodeType) -> Result<Box<dyn UiNode>, BetulaError> {
        if let Some(node_support) = self.ui_support(node_type) {
            Ok((node_support.node_factory)())
        } else {
            Err("no ui node support for {node_type:?}".into())
        }
    }
}
