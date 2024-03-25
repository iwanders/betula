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
    fn load(
        &self,
        config: &mut dyn erased_serde::Deserializer,
    ) -> Result<Box<dyn Node>, BetulaError>;
    fn store(&self, node: &dyn Node) -> Result<Box<dyn erased_serde::Serializer>, BetulaError>;

    fn reload(
        &self,
        node: &mut dyn Node,
        config: &dyn erased_serde::Deserializer,
    ) -> Result<(), BetulaError> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct DefaultLoader<T: Serialize + serde::de::DeserializeOwned + betula_core::Node + 'static> {
    z: std::marker::PhantomData<T>,
}
impl<T: Serialize + serde::de::DeserializeOwned + betula_core::Node + 'static> DefaultLoader<T> {
    pub fn new() -> Self {
        Self {
            z: std::marker::PhantomData,
        }
    }
}

impl<T: Serialize + serde::de::DeserializeOwned + betula_core::Node + 'static> NodeLoader
    for DefaultLoader<T>
{
    fn load(
        &self,
        config: &mut dyn erased_serde::Deserializer,
    ) -> Result<Box<dyn Node>, BetulaError> {
        Ok(Box::new(erased_serde::deserialize::<T>(config)?))
    }

    fn store(&self, node: &dyn Node) -> Result<Box<dyn erased_serde::Serializer>, BetulaError> {
        Err("ldskjfldsf".into())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use betula_core::prelude::*;

    #[derive(Debug, Default, Serialize, Deserialize)]
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

        let boxed_node = loader.load(&mut erased)?;

        // And cast it again:
        let loaded_dummy: &DummyNode = (*boxed_node)
            .downcast_ref::<DummyNode>()
            .ok_or("wrong type")?;

        assert!(loaded_dummy.skipped == 0.0);
        assert!(loaded_dummy.last_time == 10.0);
        assert!(loaded_dummy.interval == 3.3);
        Ok(())
    }
}
