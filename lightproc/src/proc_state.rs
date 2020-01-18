use std::any::Any;

use std::fmt::{Error, Formatter};

use std::fmt;
use std::sync::{Arc, Mutex};

pub trait State: Send + Sync + AsAny + 'static {}
impl<T> State for T where T: Send + Sync + 'static {}
impl fmt::Debug for dyn State {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.write_fmt(format_args!("State :: {:?}", self.type_id()))
    }
}

pub type ProcState = Arc<Mutex<dyn State>>;

pub trait AsAny {
    fn as_any(&mut self) -> &mut dyn Any;
}

impl<T: Any> AsAny for T {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

pub type EmptyProcState = Box<EmptyState>;
pub struct EmptyState;
