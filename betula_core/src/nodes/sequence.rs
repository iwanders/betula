use crate::prelude::*;

pub struct Sequence {}
impl Node for Sequence {
    fn tick(
        &mut self,
        self_id: NodeId,
        tree: &dyn Tree,
        ctx: &dyn Context,
    ) -> Result<Status, Error> {
        for id in tree.children(self_id) {
            match tree.run(id)? {
                Status::Running => {}
                other => return Ok(other),
            }
        }

        Ok(Status::Failure)
    }
}
