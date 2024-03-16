use crate::prelude::*;

#[derive(Debug, Copy, Clone)]
pub struct Failure {}
impl Node for Failure {
    fn tick(
        &mut self,
        _self_id: NodeId,
        _tree: &dyn Tree,
        _ctx: &mut dyn Context,
    ) -> Result<Status, Error> {
        Ok(Status::Failure)
    }
}
