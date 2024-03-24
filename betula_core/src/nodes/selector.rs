use crate::prelude::*;
use crate::{Error, Node, Status};

#[derive(Debug, Copy, Clone)]
pub struct SelectorNode {}
impl Node for SelectorNode {
    fn tick(&mut self, ctx: &dyn RunContext) -> Result<Status, Error> {
        for id in 0..ctx.children() {
            match ctx.run(id)? {
                Status::Failure => {}
                other => return Ok(other),
            }
        }

        // Reached here, all children must've failed.
        Ok(Status::Failure)
    }
}
