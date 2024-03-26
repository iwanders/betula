use betula_core::prelude::*;
use betula_core::BetulaError;
use betula_core::{Node, NodeId};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct TreeNode {
    id: NodeId,
    node_name: Option<String>,
    node_type: String,
    // Node config how?
}

#[derive(Serialize, Deserialize, Debug)]
struct Relations {
    parent: NodeId,
    children: Vec<NodeId>,
}

#[derive(Serialize, Deserialize, Debug)]
struct TreeConfig {
    nodes: Vec<TreeNode>,
    relations: Vec<Relations>,
}

trait NodeLoader {
    fn node_load(
        &self,
        config: &mut dyn erased_serde::Deserializer,
    ) -> Result<Box<dyn Node>, BetulaError>;
    fn node_store(&self, node: &dyn Node) -> Result<Box<dyn erased_serde::Serialize>, BetulaError>;
    fn node_reload(
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

impl<T: Serialize + serde::de::DeserializeOwned + betula_core::Node + 'static + Clone> NodeLoader
    for DefaultLoader<T>
{
    fn node_load(
        &self,
        config: &mut dyn erased_serde::Deserializer,
    ) -> Result<Box<dyn Node>, BetulaError> {
        Ok(Box::new(erased_serde::deserialize::<T>(config)?))
    }

    fn node_store(&self, node: &dyn Node) -> Result<Box<dyn erased_serde::Serialize>, BetulaError> {
        let v = (*node).downcast_ref::<T>().ok_or("failed to cast")?;
        Ok(Box::new((*v).clone()))
    }
    fn node_reload(
        &self,
        node: &mut dyn Node,
        config: &mut dyn erased_serde::Deserializer,
    ) -> Result<(), BetulaError> {
        // Use this nifty hidden deserialize_in_place.
        let v = (*node).downcast_mut::<T>().ok_or("failed to cast")?;
        Ok(Deserialize::deserialize_in_place(config, v)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use betula_core::prelude::*;

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

    #[test]
    fn test_things() -> Result<(), BetulaError> {
        let d = DummyNode {
            skipped: 3.3,
            last_time: 10.0,
            interval: 3.3,
        };
        let yaml = serde_yaml::to_string(&d)?;
        println!("as yaml: {yaml:?}");

        let yaml_deser = serde_yaml::Deserializer::from_str(&yaml);
        let mut erased = Box::new(<dyn erased_serde::Deserializer>::erase(yaml_deser));
        let loader: Box<dyn NodeLoader> = Box::new(DefaultLoader::<DummyNode>::new());

        let mut boxed_node = loader.node_load(&mut erased)?;

        // And cast it again:
        let loaded_dummy: &DummyNode = (*boxed_node)
            .downcast_ref::<DummyNode>()
            .ok_or("wrong type")?;

        assert!(loaded_dummy.skipped == 0.0);
        assert!(loaded_dummy.last_time == 10.0);
        assert!(loaded_dummy.interval == 3.3);

        let data = loader.node_store(&*boxed_node)?;
        let yaml_back = serde_yaml::to_string(&data)?;
        println!("as yaml: {yaml:?}");

        assert!(yaml_back == yaml);

        let mut c = d;
        c.skipped = 5.5;
        c.last_time = 12.0;
        let yaml_str = serde_yaml::to_string(&c)?;
        let yaml_deser = serde_yaml::Deserializer::from_str(&yaml_str);
        let mut erased = Box::new(<dyn erased_serde::Deserializer>::erase(yaml_deser));
        loader.node_reload(&mut *boxed_node, &mut erased)?;
        let loaded_dummy: &DummyNode = (*boxed_node)
            .downcast_ref::<DummyNode>()
            .ok_or("wrong type")?;
        println!("loaded_dummy: {loaded_dummy:?}");
        assert!(loaded_dummy.skipped == 0.0);
        assert!(loaded_dummy.last_time == 12.0);
        assert!(loaded_dummy.interval == 3.3);
        Ok(())
    }
}
