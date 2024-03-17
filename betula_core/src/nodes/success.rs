use crate::prelude::*;

#[derive(Debug, Copy, Clone)]
pub struct Success {}
impl Node for Success {
    fn tick(&mut self, _tree: &dyn Tree, _ctx: &mut dyn Context) -> Result<Status, Error> {
        Ok(Status::Success)
    }
}
