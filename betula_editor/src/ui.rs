use betula_common::{
    tree_support::SerializedBlackboardValues, tree_support::SerializedValue,
    type_support::DefaultValueRequirements, TreeSupport,
};
use egui::Ui;
use std::collections::HashMap;

use betula_core::{
    blackboard::{Chalkable, Port, PortDirection, PortName, PortType},
    BetulaError, Node, NodeType,
};

use crate::{menu_node_recurser, MenuEntry, UiMenuNode, UiMenuTree};

use std::collections::BTreeMap;

#[derive(PartialEq, Clone, Copy, Hash, Debug, Eq)]
pub enum UiConfigResponse {
    /// Config has changed, needs to be sent to the server.
    UnChanged,
    /// Config is unchanged, no action necessary.
    Changed,
}

pub trait UiNodeContext {
    fn children_count(&self) -> usize;
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

    /// The range of children this node may have.
    fn ui_child_range(&self) -> std::ops::Range<usize> {
        0..usize::MAX
    }

    /// Function to render the ui, responds whether changes were made.
    ///
    /// This should also update the configuration appropriately when the context
    /// changes.
    fn ui_config(&mut self, ctx: &dyn UiNodeContext, ui: &mut Ui, scale: f32) -> UiConfigResponse {
        let _ = (ctx, ui, scale);
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

    fn ui_category() -> Vec<UiNodeCategory>
    where
        Self: Sized,
    {
        vec![UiNodeCategory::Name(Self::static_type().into())]
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
    pub display_name: String,
    pub value_factory: UiValueFactory,
}

#[derive(Clone, Hash, Debug, Ord, Eq, PartialEq, PartialOrd)]
pub enum UiNodeCategory {
    Group(String),
    Folder(String),
    Name(String),
}
impl UiNodeCategory {
    pub fn name(&self) -> &str {
        match self {
            UiNodeCategory::Group(v) => v.as_str(),
            UiNodeCategory::Folder(v) => v.as_str(),
            UiNodeCategory::Name(v) => v.as_str(),
        }
    }
}

pub trait UiValue: std::fmt::Debug {
    /// Function to render the ui, responds whether changes were made.
    fn ui(&mut self, _ui: &mut Ui, _scale: f32) -> UiConfigResponse {
        UiConfigResponse::UnChanged
    }

    fn value(&self) -> Box<dyn Chalkable>;
    fn set_value(&mut self, value: Box<dyn Chalkable>) -> Result<(), BetulaError>;

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
    fn set_value(&mut self, boxed_value: Box<dyn Chalkable>) -> Result<(), BetulaError> {
        use betula_core::as_any::AsAnyHelper;
        let new_value = (*boxed_value).downcast_ref::<T>();
        if let Some(v) = new_value {
            self.data = v.clone();
            Ok(())
        } else {
            Err(format!(
                "could not downcast {:?} to {:?}",
                (*boxed_value).as_any_type_name(),
                std::any::type_name::<T>()
            )
            .into())
        }
    }
    fn value_type(&self) -> String {
        Self::static_type()
    }
    fn static_type() -> String {
        std::any::type_name::<T>().to_owned()
    }
}

type UiCategoryTree = UiMenuTree<String, NodeType>;
type UiCategoryNode = UiMenuNode<String, NodeType>;

pub struct UiSupport {
    ui_node: HashMap<NodeType, UiNodeSupport>,
    ui_value: HashMap<String, UiValueSupport>,
    tree: TreeSupport,

    node_categories: UiCategoryTree,
}

impl UiSupport {
    pub fn new() -> Self {
        Self {
            ui_value: Default::default(),
            ui_node: Default::default(),
            tree: Default::default(),
            node_categories: Default::default(),
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

    pub fn node_categories(&self) -> &UiCategoryTree {
        &self.node_categories
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

        // Go from categories to the tree;
        let category = T::ui_category();
        let mut current = &mut self.node_categories;
        for c in category {
            match c {
                UiNodeCategory::Group(ref g) => {
                    current = current
                        .entry(g.to_owned())
                        .or_insert_with(|| UiCategoryNode::Groups(UiCategoryTree::new()))
                        .groups()
                }
                UiNodeCategory::Folder(ref v) => {
                    current = current
                        .entry(v.to_owned())
                        .or_insert_with(|| UiCategoryNode::SubElements(UiCategoryTree::new()))
                        .sub_elements()
                }
                UiNodeCategory::Name(ref v) => {
                    current.insert(v.to_owned(), UiCategoryNode::Value(T::static_type()));
                }
            }
        }
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

    pub fn add_value_default_named<V: betula_common::type_support::DefaultValueRequirements>(
        &mut self,
        name: &str,
    ) {
        self.tree.add_value_default::<V>();
        use betula_core::as_any::AsAnyHelper;
        let name = name.to_owned();
        let name_for_closure = name.clone();
        let value_support = UiValueSupport {
            type_id: std::any::type_name::<V>().to_owned(),
            display_name: name.to_owned(),
            value_factory: Box::new(move |tree_support: &TreeSupport, v: SerializedValue| {
                let z = tree_support.value_deserialize(v.clone())?;
                if let Some(v) = (*z).downcast_ref::<V>() {
                    Ok(Box::new(DefaultUiValueHandler::<V> { data: v.clone() }))
                } else {
                    Err(format!("failed to downcast {v:?} to {name_for_closure:?}").into())
                }
            }),
        };
        self.ui_value
            .insert(std::any::type_name::<V>().to_owned(), value_support);
    }

    pub fn add_value_default<V: betula_common::type_support::DefaultValueRequirements>(&mut self) {
        self.add_value_default_named::<V>(std::any::type_name::<V>());
    }

    pub fn add_value_custom<V: DefaultValueRequirements>(&mut self, value_support: UiValueSupport) {
        self.tree.add_value_default::<V>();
        self.ui_value
            .insert(std::any::type_name::<V>().to_owned(), value_support);
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
            Err(format!("no ui node support for {node_type:?}").into())
        }
    }

    pub fn create_ui_value(&self, value: SerializedValue) -> Result<Box<dyn UiValue>, BetulaError> {
        let value_type = &value.type_id;
        if let Some(value_support) = self.value_support(&value_type) {
            Ok((value_support.value_factory)(&self.tree, value)?)
        } else {
            Err(format!("no ui value support for {value_type:?}").into())
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

    pub fn port_display_name(&self, port_type: &PortType) -> String {
        if let Some(value_support) = self.value_support(port_type.type_name()) {
            value_support.display_name.clone()
        } else {
            format!("{port_type:?}")
        }
    }
}
