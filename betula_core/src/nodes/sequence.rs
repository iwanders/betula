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
                Status::Success => {}
                other => return Ok(other), // fail or running.
            }
        }

        // All children succeeded.
        Ok(Status::Success)
    }
}
