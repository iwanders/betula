use crate::prelude::*;
use crate::{Error, Node, Status};

#[derive(Debug, Copy, Clone)]
pub struct SuccessNode {}
impl Node for SuccessNode {
    fn tick(&mut self, _ctx: &dyn RunContext) -> Result<Status, Error> {
        Ok(Status::Success)
    }
}
