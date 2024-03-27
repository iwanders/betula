use betula_core::prelude::*;
use betula_core::BetulaError;
use betula_core::Node;
use serde::{Deserialize, Serialize};

/// Trait to create nodes out of thin air.
pub trait NodeFactory {
    fn create(&self) -> Result<Box<dyn Node>, BetulaError>;
}

/// Trait to facilitate
trait NodeConfigLoader {
    fn get_config(&self, node: &dyn Node) -> Result<Box<dyn erased_serde::Serialize>, BetulaError>;
    fn set_config(
        &self,
        node: &mut dyn Node,
        config: &mut dyn erased_serde::Deserializer,
    ) -> Result<(), BetulaError>;
}

#[derive(Debug)]
pub struct DefaultLoader<T: Serialize + serde::de::DeserializeOwned + betula_core::Node + 'static> {
    _z: std::marker::PhantomData<T>,
}
impl<T: Serialize + serde::de::DeserializeOwned + betula_core::Node + 'static> DefaultLoader<T> {
    pub fn new() -> Self {
        Self {
            _z: std::marker::PhantomData,
        }
    }
}

impl<T: Serialize + serde::de::DeserializeOwned + betula_core::Node + 'static + Clone>
    NodeConfigLoader for DefaultLoader<T>
{
    fn get_config(&self, node: &dyn Node) -> Result<Box<dyn erased_serde::Serialize>, BetulaError> {
        let v = (*node).downcast_ref::<T>().ok_or("failed to cast")?;
        Ok(Box::new((*v).clone()))
    }
    fn set_config(
        &self,
        node: &mut dyn Node,
        config: &mut dyn erased_serde::Deserializer,
    ) -> Result<(), BetulaError> {
        // Use this nifty hidden deserialize_in_place to avoid overwriting skipped values.
        let v = (*node).downcast_mut::<T>().ok_or("failed to cast")?;
        Ok(Deserialize::deserialize_in_place(config, v)?)
    }
}

impl<T: Serialize + serde::de::DeserializeOwned + betula_core::Node + 'static + Default> NodeFactory
    for DefaultLoader<T>
{
    fn create(&self) -> Result<Box<dyn Node>, BetulaError> {
        Ok(Box::new(T::default()))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Debug, Default, Serialize, Deserialize, Clone)]
    pub struct DummyNode {
        #[serde(skip)]
        skipped: f32,
        last_time: f64,
        interval: f64,
    }
    impl Node for DummyNode {
        fn tick(
            &mut self,
            _: &dyn betula_core::RunContext,
        ) -> Result<betula_core::NodeStatus, BetulaError> {
            todo!()
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct StructWithYaml {
        node_name: Option<String>,
        node_type: String,
        // Node config how?
        config: serde_yaml::Value,
    }

    #[test]
    fn test_things() -> Result<(), BetulaError> {
        let d = DummyNode {
            skipped: 3.3,
            last_time: 10.0,
            interval: 3.3,
        };
        let yaml = serde_yaml::to_string(&d)?;
        println!("as yaml: {yaml:?}");

        let factory: Box<dyn NodeFactory> = Box::new(DefaultLoader::<DummyNode>::new());

        let boxed_node = factory.create()?;

        // And cast it again:
        let loaded_dummy: &DummyNode = (*boxed_node)
            .downcast_ref::<DummyNode>()
            .ok_or("wrong type")?;

        assert!(loaded_dummy.skipped == 0.0);
        assert!(loaded_dummy.last_time == 0.0);
        assert!(loaded_dummy.interval == 0.0);

        let loader: Box<dyn NodeConfigLoader> = Box::new(DefaultLoader::<DummyNode>::new());
        let mut boxed_node = factory.create()?;
        let yaml_deser = serde_yaml::Deserializer::from_str(&yaml);
        let mut erased = Box::new(<dyn erased_serde::Deserializer>::erase(yaml_deser));
        let _ = loader.set_config(&mut *boxed_node, &mut erased)?;

        let loaded_dummy: &DummyNode = (*boxed_node)
            .downcast_ref::<DummyNode>()
            .ok_or("wrong type")?;
        assert!(loaded_dummy.skipped == 0.0);
        assert!(loaded_dummy.last_time == 10.0);
        assert!(loaded_dummy.interval == 3.3);

        let data = loader.get_config(&*boxed_node)?;
        let yaml_back = serde_yaml::to_string(&data)?;
        println!("as yaml: {yaml:?}");

        assert!(yaml_back == yaml);

        let mut c = d;
        c.skipped = 5.5;
        c.last_time = 12.0;
        let yaml_str = serde_yaml::to_string(&c)?;
        let yaml_deser = serde_yaml::Deserializer::from_str(&yaml_str);
        let mut erased = Box::new(<dyn erased_serde::Deserializer>::erase(yaml_deser));
        loader.set_config(&mut *boxed_node, &mut erased)?;
        let loaded_dummy: &DummyNode = (*boxed_node)
            .downcast_ref::<DummyNode>()
            .ok_or("wrong type")?;
        println!("loaded_dummy: {loaded_dummy:?}");
        assert!(loaded_dummy.skipped == 0.0);
        assert!(loaded_dummy.last_time == 12.0);
        assert!(loaded_dummy.interval == 3.3);

        let config = serde_yaml::from_str("x: 1.0\ny: 2.0\n")?;
        let z = StructWithYaml {
            node_name: None,
            node_type: "foo".to_owned(),
            // Node config how?
            config,
        };
        let yaml_str = serde_yaml::to_string(&z)?;
        println!("as yaml: {yaml_str}");

        Ok(())
    }
}
