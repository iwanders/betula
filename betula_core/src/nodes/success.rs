use crate::prelude::*;

#[derive(Debug, Copy, Clone)]
pub struct Success {}
impl Node for Success {
    fn tick(&mut self, _ctx: &dyn RunContext) -> Result<Status, Error> {
        Ok(Status::Success)
    }
}
