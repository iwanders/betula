use crate::prelude::*;

#[derive(Debug, Copy, Clone)]
pub struct Sequence {}
impl Node for Sequence {
    fn tick(
        &mut self,
        self_id: NodeId,
        tree: &dyn Tree,
        ctx: &mut dyn Context,
    ) -> Result<Status, Error> {
        for id in tree.children(self_id) {
            match tree.run(id)? {
                Status::Running => {}
                other => return Ok(other),
            }
        }

        // No children, should this be an error instead?
        Ok(Status::Failure)
    }
}
