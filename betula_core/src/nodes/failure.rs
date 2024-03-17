use crate::prelude::*;

#[derive(Debug, Copy, Clone)]
pub struct Failure {}
impl Node for Failure {
    fn tick(&mut self, _ctx: &dyn RunContext) -> Result<Status, Error> {
        Ok(Status::Failure)
    }
}
