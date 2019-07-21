use std::any::Any;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PoisonPill;

impl PoisonPill {
    pub fn new() -> Box<PoisonPill> {
        Box::new(PoisonPill::default())
    }

    pub fn as_any(&self) -> &dyn Any {
        self
    }
}
