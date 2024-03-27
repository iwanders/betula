use betula_core::prelude::*;
use betula_core::BetulaError;
use betula_core::{Node, NodeConfig, NodeError};
use serde::{Deserialize, Serialize};

/// Trait to create nodes out of thin air.
pub trait NodeFactory {
    fn create(&self) -> Result<Box<dyn Node>, BetulaError>;
}

/// Default factory for nodes.
#[derive(Debug)]
pub struct DefaultFactory<T: betula_core::Node + 'static + Default> {
    _z: std::marker::PhantomData<T>,
}
impl<T: betula_core::Node + 'static + Default> DefaultFactory<T> {
    pub fn new() -> Self {
        Self {
            _z: std::marker::PhantomData,
        }
    }
}
impl<T: betula_core::Node + 'static + Default> NodeFactory for DefaultFactory<T> {
    fn create(&self) -> Result<Box<dyn Node>, BetulaError> {
        Ok(Box::new(T::default()))
    }
}

/// Trait to facilitate serialization and deserialization of configs.
trait ConfigConverter {
    fn config_serialize(
        &self,
        config: &dyn NodeConfig,
    ) -> Result<Box<dyn erased_serde::Serialize>, BetulaError>;
    fn config_deserialize(
        &self,
        config: &mut dyn erased_serde::Deserializer,
    ) -> Result<Box<dyn NodeConfig>, BetulaError>;
}

/// Default config converter
#[derive(Debug)]
pub struct DefaultConfigConverter<T: Serialize + serde::de::DeserializeOwned + 'static> {
    _z: std::marker::PhantomData<T>,
}
impl<T: Serialize + serde::de::DeserializeOwned + 'static> DefaultConfigConverter<T> {
    pub fn new() -> Self {
        Self {
            _z: std::marker::PhantomData,
        }
    }
}
impl<T: Serialize + serde::de::DeserializeOwned + 'static + std::fmt::Debug + Clone> ConfigConverter
    for DefaultConfigConverter<T>
{
    fn config_serialize(
        &self,
        config: &dyn NodeConfig,
    ) -> Result<Box<dyn erased_serde::Serialize>, BetulaError> {
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

#[cfg(test)]
mod test {
    use super::*;
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
    }

    #[test]
    fn test_things() -> Result<(), BetulaError> {
        let factory: Box<dyn NodeFactory> = Box::new(DefaultFactory::<DummyNode>::new());
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
