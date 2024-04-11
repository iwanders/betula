use betula_common::{
    tree_support::SerializedBlackboardValues, tree_support::SerializedValue, TreeSupport,
};
use egui::Ui;
use std::collections::HashMap;

use betula_core::{
    blackboard::{Chalkable, Port, PortDirection, PortName},
    BetulaError, Node, NodeType,
};

use std::collections::BTreeMap;

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

    /// The input to show at this input number.
    fn ui_input_port(&self, input: usize) -> Option<Port> {
        self.ports()
            .unwrap_or(vec![])
            .iter()
            .filter(|p| p.direction() == PortDirection::Input)
            .nth(input)
            .cloned()
    }

    /// The output port to show at this output number.
    fn ui_output_port(&self, output: usize) -> Option<Port> {
        self.ports()
            .unwrap_or(vec![])
            .iter()
            .filter(|p| p.direction() == PortDirection::Output)
            .nth(output)
            .cloned()
    }

    fn ui_port_output(&self, name: &PortName) -> Option<usize> {
        self.ports()
            .unwrap_or(vec![])
            .iter()
            .filter(|p| p.direction() == PortDirection::Output)
            .position(|x| x.name() == *name)
    }

    fn ui_port_input(&self, name: &PortName) -> Option<usize> {
        self.ports()
            .unwrap_or(vec![])
            .iter()
            .filter(|p| p.direction() == PortDirection::Input)
            .position(|x| x.name() == *name)
    }
}

type UiNodeFactory = Box<dyn Fn() -> Box<dyn UiNode>>;
pub struct UiNodeSupport {
    pub display_name: String,
    pub node_factory: UiNodeFactory,
}

type UiValueFactory =
    Box<dyn Fn(&TreeSupport, SerializedValue) -> Result<Box<dyn UiValue>, BetulaError>>;
pub struct UiValueSupport {
    pub type_id: String,
    pub value_factory: UiValueFactory,
}

pub trait UiValue: std::fmt::Debug {
    /// Function to render the ui, responds whether changes were made.
    fn ui(&mut self, _ui: &mut Ui, _scale: f32) -> UiConfigResponse {
        UiConfigResponse::UnChanged
    }

    fn value(&self) -> Box<dyn Chalkable>;

    fn value_type(&self) -> String;

    fn static_type() -> String
    where
        Self: Sized;
}

#[derive(Debug)]
struct DefaultUiValueHandler<T: Chalkable + std::fmt::Debug> {
    data: T,
}
impl<T: Chalkable + std::fmt::Debug + Clone + 'static> UiValue for DefaultUiValueHandler<T> {
    fn ui(&mut self, ui: &mut Ui, _scale: f32) -> UiConfigResponse {
        ui.label(format!("{:.?}", self.data));
        UiConfigResponse::UnChanged
    }
    fn value(&self) -> Box<dyn Chalkable> {
        Box::new(self.data.clone())
    }
    fn value_type(&self) -> String {
        Self::static_type()
    }
    fn static_type() -> String {
        std::any::type_name::<T>().to_owned()
    }
}

pub struct UiSupport {
    ui_node: HashMap<NodeType, UiNodeSupport>,
    ui_value: HashMap<String, UiValueSupport>,
    tree: TreeSupport,
}

impl UiSupport {
    pub fn new() -> Self {
        Self {
            ui_value: Default::default(),
            ui_node: Default::default(),
            tree: Default::default(),
        }
    }

    pub fn into_tree_support(self) -> TreeSupport {
        self.tree
    }

    pub fn tree_support_ref(&self) -> &TreeSupport {
        &self.tree
    }

    pub fn tree_support_mut(&mut self) -> &mut TreeSupport {
        &mut self.tree
    }

    pub fn node_support(&self, node_type: &NodeType) -> Option<&UiNodeSupport> {
        self.ui_node.get(node_type)
    }
    pub fn value_support(&self, value_type: &str) -> Option<&UiValueSupport> {
        self.ui_value.get(value_type)
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
        self.ui_node.insert(T::static_type(), ui_support);
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

    pub fn add_value_default<V: betula_common::type_support::DefaultValueRequirements>(&mut self) {
        self.tree.add_value_default::<V>();
        use betula_core::as_any::AsAnyHelper;
        let name = std::any::type_name::<V>().to_owned();
        let name_for_closure = name.clone();
        let value_support = UiValueSupport {
            type_id: name.clone(),
            value_factory: Box::new(move |tree_support: &TreeSupport, v: SerializedValue| {
                let z = tree_support.value_deserialize(v.clone())?;
                if let Some(v) = (*z).downcast_ref::<V>() {
                    Ok(Box::new(DefaultUiValueHandler::<V> { data: v.clone() }))
                } else {
                    Err(format!("failed to downcast {v:?} to {name_for_closure:?}").into())
                }
            }),
        };
        self.ui_value.insert(name, value_support);
    }

    /*
    pub fn set_blackboard_factory(&mut self, blackboard_factory: betula_common::tree_support::BlackboardFactory) {
        self.tree.set_blackboard_factory(blackboard_factory);
    }
    */

    // pub fn create_blackboard(&self) -> Option<Box<dyn Blackboard>> {
    // self.tree.create_blackboard()
    // }

    pub fn node_types(&self) -> Vec<NodeType> {
        self.ui_node.keys().cloned().collect()
    }

    pub fn display_name(&self, node_type: &NodeType) -> String {
        if let Some(node_support) = self.node_support(node_type) {
            node_support.display_name.clone()
        } else {
            "Unknown Node".into()
        }
    }

    pub fn create_ui_node(&self, node_type: &NodeType) -> Result<Box<dyn UiNode>, BetulaError> {
        if let Some(node_support) = self.node_support(node_type) {
            Ok((node_support.node_factory)())
        } else {
            Err("no ui node support for {node_type:?}".into())
        }
    }

    pub fn create_ui_value(&self, value: SerializedValue) -> Result<Box<dyn UiValue>, BetulaError> {
        let value_type = &value.type_id;
        if let Some(value_support) = self.value_support(&value_type) {
            Ok((value_support.value_factory)(&self.tree, value)?)
        } else {
            Err("no ui value support for {value_type:?}".into())
        }
    }

    pub fn create_ui_values(
        &self,
        port_values: &SerializedBlackboardValues,
    ) -> Result<BTreeMap<PortName, Box<dyn UiValue>>, BetulaError> {
        let mut res: BTreeMap<PortName, Box<dyn UiValue>> = Default::default();
        for (k, v) in port_values {
            res.insert(k.clone(), self.create_ui_value(v.clone())?);
        }
        Ok(res)
    }
}
