use crate::prelude::*;
use crate::{Error, Node, Status};

#[derive(Debug, Copy, Clone)]
pub struct FailureNode {}
impl Node for FailureNode {
    fn tick(&mut self, _ctx: &dyn RunContext) -> Result<Status, Error> {
        Ok(Status::Failure)
    }
}
