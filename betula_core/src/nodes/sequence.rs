use crate::prelude::*;
use crate::{Error, Node, Status};

#[derive(Debug, Copy, Clone)]
pub struct SequenceNode {}
impl Node for SequenceNode {
    fn tick(&mut self, ctx: &dyn RunContext) -> Result<Status, Error> {
        for id in 0..ctx.children() {
            match ctx.run(id)? {
                Status::Success => {}
                other => return Ok(other), // fail or running.
            }
        }

        // All children succeeded.
        Ok(Status::Success)
    }
}
