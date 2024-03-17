use crate::prelude::*;

#[derive(Debug, Copy, Clone)]
pub struct Fallback {}
impl Node for Fallback {
    fn tick(&mut self, tree: &dyn Tree, ctx: &mut dyn Context) -> Result<Status, Error> {
        for id in 0..tree.children() {
            match tree.run(id)? {
                Status::Failure => {}
                other => return Ok(other),
            }
        }

        // Reached here, all children must've failed.
        Ok(Status::Failure)
    }
}
