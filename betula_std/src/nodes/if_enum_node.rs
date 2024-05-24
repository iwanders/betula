use betula_core::node_prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[cfg(feature = "betula_editor")]
use betula_editor::UiNodeCategory;

/// Trait that enums that want to use this node must implement.
pub trait IfEnumNodeEnum:
    PartialEq + Serialize + DeserializeOwned + Clone + std::fmt::Debug + 'static + Send + Sized
{
    /// This is the snake case representation of the type.
    ///
    /// This is both used in the title, as well as in the node's static type used for serialization.
    fn enum_node_name() -> &'static str;

    /// The enums as selectable from the drop down.
    fn enum_node_enumeration() -> &'static [Self];

    /// The default value used for the configuration.
    fn enum_node_default() -> Self;

    /// The node category in the ui.
    #[cfg(feature = "betula_editor")]
    fn enum_node_category() -> Vec<UiNodeCategory> {
        vec![
            UiNodeCategory::Folder("decorator".to_owned()),
            UiNodeCategory::Name(format!("if_{}", Self::enum_node_name())),
        ]
    }
}

/// Example node for the [`ExecutionStatus`] type.
pub type IfExecutionStatusNode = IfEnumNode<ExecutionStatus>;
/// Example node config for the [`ExecutionStatus`] type.
pub type IfExecutionStatusNodeConfig = IfEnumNodeConfig<ExecutionStatus>;

/// And its trait implementation.
impl IfEnumNodeEnum for ExecutionStatus {
    fn enum_node_name() -> &'static str {
        "execution_status"
    }
    fn enum_node_default() -> Self {
        ExecutionStatus::Running
    }
    fn enum_node_enumeration() -> &'static [Self] {
        &[
            ExecutionStatus::Running,
            ExecutionStatus::Success,
            ExecutionStatus::Failure,
        ]
    }
}

/// The comparison type for the node enum.
#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub enum IfEnumNodeComparison {
    #[default]
    Equal,
    NotEqual,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound = "T: Serialize + DeserializeOwned")]
pub struct IfEnumNodeConfig<T: IfEnumNodeEnum> {
    pub value: T,
    pub comparison: IfEnumNodeComparison,
}
impl<T: IfEnumNodeEnum> Default for IfEnumNodeConfig<T> {
    fn default() -> Self {
        Self {
            value: T::enum_node_default(),
            comparison: Default::default(),
        }
    }
}

impl<T: IfEnumNodeEnum> IsNodeConfig for IfEnumNodeConfig<T> {}

#[derive(Debug)]
pub struct IfEnumNode<T: IfEnumNodeEnum> {
    input: Input<T>,
    pub config: IfEnumNodeConfig<T>,
}
impl<T: IfEnumNodeEnum> Default for IfEnumNode<T> {
    fn default() -> Self {
        Self {
            input: Default::default(),
            config: Default::default(),
        }
    }
}

impl<T: IfEnumNodeEnum> Node for IfEnumNode<T> {
    fn execute(&mut self, ctx: &dyn RunContext) -> Result<ExecutionStatus, NodeError> {
        let value = self.input.get()?;
        let boolean_value = if self.config.comparison == IfEnumNodeComparison::Equal {
            value == self.config.value
        } else {
            value != self.config.value
        };
        if boolean_value {
            ctx.decorate_or(ExecutionStatus::Success)
        } else {
            Ok(ExecutionStatus::Failure)
        }
    }

    fn ports(&self) -> Result<Vec<Port>, NodeError> {
        Ok(vec![Port::input::<T>(T::enum_node_name())])
    }

    fn setup_inputs(
        &mut self,
        interface: &mut dyn BlackboardInputInterface,
    ) -> Result<(), NodeError> {
        self.input = interface.input::<T>(T::enum_node_name())?;
        Ok(())
    }

    fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
        Ok(Some(Box::new(self.config.clone())))
    }

    fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
        self.config.load_node_config(config)
    }

    fn reset(&mut self) {}

    fn static_type() -> NodeType {
        format!("if_enum_{}", T::enum_node_name()).into()
    }

    fn node_type(&self) -> NodeType {
        Self::static_type()
    }
}
#[cfg(feature = "betula_editor")]
pub mod ui_support {
    use super::*;
    use betula_editor::{egui, UiConfigResponse, UiNode, UiNodeCategory, UiNodeContext};

    impl<T: IfEnumNodeEnum> UiNode for IfEnumNode<T> {
        fn ui_title(&self) -> String {
            format!("if_{}", T::enum_node_name())
        }
        fn ui_icon(&self, ui: &mut egui::Ui, desired_size: egui::Vec2) {
            let _ = desired_size;
            ui.add(egui::Label::new("🔱").selectable(false));
        }

        fn ui_config(
            &mut self,
            ctx: &dyn UiNodeContext,
            ui: &mut egui::Ui,
            _scale: f32,
        ) -> UiConfigResponse {
            let _ = ctx;
            let mut ui_response = UiConfigResponse::UnChanged;

            let mut cmp_options = vec![];
            let cmp_order = [IfEnumNodeComparison::Equal, IfEnumNodeComparison::NotEqual];
            let cmp_str = ["==", "!="];
            let mut index = 0;
            for (i, v) in cmp_order.iter().enumerate() {
                cmp_options.push((i, cmp_str[i], *v));
                if *v == self.config.comparison {
                    index = i;
                }
            }
            let z = egui::ComboBox::from_id_source(0)
                .width(0.0)
                .selected_text(cmp_str[index])
                .show_index(ui, &mut index, cmp_options.len(), |i| cmp_options[i].1);
            if z.changed() {
                self.config.comparison = cmp_options[index].2.clone();
                ui_response = UiConfigResponse::Changed;
            }

            let mut options = vec![];
            let mut index = 0;
            for (i, v) in T::enum_node_enumeration().iter().enumerate() {
                options.push((i, format!("{v:?}"), v.clone()));
                if *v == self.config.value {
                    index = i;
                }
            }

            let z = egui::ComboBox::from_id_source(1)
                .width(0.0)
                .selected_text(format!("{:?}", options[index].1))
                .show_index(ui, &mut index, options.len(), |i| &options[i].1);
            if z.changed() {
                self.config.value = options[index].2.clone();
                ui_response = UiConfigResponse::Changed;
            }

            ui_response
        }
        fn ui_child_range(&self) -> std::ops::Range<usize> {
            1..1
        }

        fn ui_category() -> Vec<UiNodeCategory> {
            T::enum_node_category()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[derive(PartialEq, Serialize, Deserialize, Clone, Debug, Default)]
    enum Foo {
        #[default]
        Bar,
        Buz,
    }
    impl IfEnumNodeEnum for Foo {
        fn enum_node_name() -> &'static str
        where
            Self: Sized,
        {
            "foo"
        }
        fn enum_node_enumeration() -> &'static [Self] {
            &[Foo::Bar, Foo::Buz]
        }
    }

    #[test]
    fn test_if_enum_foo() {
        let z: IfEnumNode<Foo> = Default::default();
    }
}
