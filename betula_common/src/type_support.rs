use betula_core::prelude::*;
use betula_core::BetulaError;
use betula_core::{Node, NodeConfig};
use serde::Serialize;

/// Trait to create nodes out of thin air.
pub trait NodeFactory: std::fmt::Debug {
    fn create(&self) -> Result<Box<dyn Node>, BetulaError>;
}

pub trait DefaultNodeFactoryRequirements: betula_core::Node + 'static + Default {}
impl<T> DefaultNodeFactoryRequirements for T where T: betula_core::Node + 'static + Default {}

/// Default factory for nodes.
pub struct DefaultNodeFactory<T: DefaultNodeFactoryRequirements> {
    _z: std::marker::PhantomData<T>,
}
impl<T: DefaultNodeFactoryRequirements> std::fmt::Debug for DefaultNodeFactory<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "DefaultNodeFactory<{}>", std::any::type_name::<T>())
    }
}

impl<T: DefaultNodeFactoryRequirements> DefaultNodeFactory<T> {
    pub fn new() -> Self {
        Self {
            _z: std::marker::PhantomData,
        }
    }
}
impl<T: DefaultNodeFactoryRequirements> NodeFactory for DefaultNodeFactory<T> {
    fn create(&self) -> Result<Box<dyn Node>, BetulaError> {
        Ok(Box::new(T::default()))
    }
}

/// Trait to facilitate serialization and deserialization of configs.
pub trait ConfigConverter: std::fmt::Debug {
    fn config_serialize(
        &self,
        config: &dyn NodeConfig,
    ) -> Result<Box<dyn erased_serde::Serialize>, BetulaError>;
    fn config_deserialize(
        &self,
        config: &mut dyn erased_serde::Deserializer,
    ) -> Result<Box<dyn NodeConfig>, BetulaError>;
}

pub trait DefaultConfigRequirements:
    Serialize + serde::de::DeserializeOwned + 'static + std::fmt::Debug + Clone + Send
{
}
impl<T> DefaultConfigRequirements for T where
    T: Serialize + serde::de::DeserializeOwned + 'static + std::fmt::Debug + Clone + Send
{
}

/// Default config converter
pub struct DefaultConfigConverter<T: DefaultConfigRequirements> {
    _z: std::marker::PhantomData<T>,
}
impl<T: DefaultConfigRequirements> std::fmt::Debug for DefaultConfigConverter<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            fmt,
            "DefaultConfigConverter<{}>",
            std::any::type_name::<T>()
        )
    }
}

impl<T: DefaultConfigRequirements> DefaultConfigConverter<T> {
    pub fn new() -> Self {
        Self {
            _z: std::marker::PhantomData,
        }
    }
}
impl<T: DefaultConfigRequirements> ConfigConverter for DefaultConfigConverter<T> {
    fn config_serialize(
        &self,
        config: &dyn NodeConfig,
    ) -> Result<Box<dyn erased_serde::Serialize>, BetulaError> {
        use betula_core::as_any::AsAnyHelper;
        let v = (*config).downcast_ref::<T>().ok_or("failed to cast")?;
        Ok(Box::new((*v).clone()))
    }
    fn config_deserialize(
        &self,
        config: &mut dyn erased_serde::Deserializer,
    ) -> Result<Box<dyn NodeConfig>, BetulaError> {
        Ok(Box::new(erased_serde::deserialize::<T>(config)?))
    }
}
use betula_core::blackboard::Chalkable;

/// Trait to facilitate serialization and deserialization of blackboard values.
pub trait ValueConverter: std::fmt::Debug {
    fn value_serialize(
        &self,
        config: &dyn Chalkable,
    ) -> Result<Box<dyn erased_serde::Serialize>, BetulaError>;
    fn value_deserialize(
        &self,
        config: &mut dyn erased_serde::Deserializer,
    ) -> Result<Box<dyn Chalkable>, BetulaError>;
}

pub trait DefaultValueRequirements:
    Serialize + serde::de::DeserializeOwned + 'static + Chalkable + Clone
{
}
impl<T> DefaultValueRequirements for T where
    T: Serialize + serde::de::DeserializeOwned + 'static + Chalkable + Clone
{
}

/// Default value converter
pub struct DefaultValueConverter<T: DefaultValueRequirements> {
    _z: std::marker::PhantomData<T>,
}
impl<T: DefaultValueRequirements> std::fmt::Debug for DefaultValueConverter<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "DefaultValueConverter<{}>", std::any::type_name::<T>())
    }
}

impl<T: DefaultValueRequirements> DefaultValueConverter<T> {
    pub fn new() -> Self {
        Self {
            _z: std::marker::PhantomData,
        }
    }
}
impl<T: DefaultValueRequirements> ValueConverter for DefaultValueConverter<T> {
    fn value_serialize(
        &self,
        value: &dyn Chalkable,
    ) -> Result<Box<dyn erased_serde::Serialize>, BetulaError> {
        use betula_core::as_any::AsAnyHelper;
        let v = (*value)
            .downcast_ref::<T>()
            .ok_or("failed to cast")?
            .clone();
        Ok(Box::new(v.clone()))
    }
    fn value_deserialize(
        &self,
        value: &mut dyn erased_serde::Deserializer,
    ) -> Result<Box<dyn Chalkable>, BetulaError> {
        Ok(Box::new(erased_serde::deserialize::<T>(value)?))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use betula_core::{NodeError, NodeType};
    use serde::Deserialize;

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct DummyConfig {
        nonzero: f32,
        interval: f64,
    }

    impl Default for DummyConfig {
        fn default() -> Self {
            Self {
                nonzero: 1337.0,
                interval: 0.5,
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct DummyNode {
        last_time: f64,
        config: DummyConfig,
    }

    impl Node for DummyNode {
        fn tick(
            &mut self,
            _: &dyn betula_core::RunContext,
        ) -> Result<betula_core::NodeStatus, BetulaError> {
            todo!()
        }

        fn get_config(&self) -> Result<Option<Box<dyn NodeConfig>>, NodeError> {
            Ok(Some(Box::new(self.config.clone())))
        }
        fn set_config(&mut self, config: &dyn NodeConfig) -> Result<(), NodeError> {
            let v = (*config)
                .downcast_ref::<DummyConfig>()
                .ok_or("failed to cast")?;
            self.config = v.clone();
            Ok(())
        }
        fn static_type() -> NodeType
        where
            Self: Sized,
        {
            "dummy".into()
        }

        fn node_type(&self) -> NodeType {
            Self::static_type()
        }
    }

    #[test]
    fn test_things() -> Result<(), BetulaError> {
        let factory: Box<dyn NodeFactory> = Box::new(DefaultNodeFactory::<DummyNode>::new());
        let mut boxed_node = factory.create()?;
        {
            let loaded_dummy: &DummyNode = (*boxed_node)
                .downcast_ref::<DummyNode>()
                .ok_or("wrong type")?;
            assert!(loaded_dummy.config.nonzero == 1337.0);
            assert!(loaded_dummy.config.interval == 0.5);
            assert!(loaded_dummy.last_time == 0.0);
        }

        // Lets make ourselves the config converter
        let converter: Box<dyn ConfigConverter> =
            Box::new(DefaultConfigConverter::<DummyConfig>::new());

        let our_new_config: Box<dyn NodeConfig> = Box::new(DummyConfig {
            nonzero: 5.3,
            interval: 3.3,
        });

        // Verify setting the config.
        let _ = boxed_node.set_config(&*our_new_config)?;
        {
            let loaded_dummy: &DummyNode = (*boxed_node)
                .downcast_ref::<DummyNode>()
                .ok_or("wrong type")?;
            assert_eq!(loaded_dummy.config.nonzero, 5.3);
            assert_eq!(loaded_dummy.config.interval, 3.3);
            assert_eq!(loaded_dummy.last_time, 0.0);
        }

        let our_newer_config_input = DummyConfig {
            nonzero: 50.3,
            interval: 30.3,
        };
        let our_newer_config: Box<dyn NodeConfig> = Box::new(our_newer_config_input.clone());

        let serializable = converter.config_serialize(&*our_newer_config)?;
        let config_json = serde_json::to_string(&serializable)?;
        let input_config_json = serde_json::to_string(&our_newer_config_input)?;
        assert_eq!(config_json, input_config_json);

        // Convert the string back to a NodeConfig
        let mut json_deser = serde_json::Deserializer::from_str(&config_json);
        let mut erased = Box::new(<dyn erased_serde::Deserializer>::erase(&mut json_deser));
        let new_config = converter.config_deserialize(&mut *erased)?;
        {
            let config: &DummyConfig = (*new_config)
                .downcast_ref::<DummyConfig>()
                .ok_or("wrong type")?;
            assert_eq!(config.nonzero, 50.3);
            assert_eq!(config.interval, 30.3);
        }

        // Finally, load the config.
        let _ = boxed_node.set_config(&*new_config)?;
        {
            let loaded_dummy: &DummyNode = (*boxed_node)
                .downcast_ref::<DummyNode>()
                .ok_or("wrong type")?;
            assert_eq!(loaded_dummy.config.nonzero, 50.3);
            assert_eq!(loaded_dummy.config.interval, 30.3);
        }

        Ok(())
    }
}
