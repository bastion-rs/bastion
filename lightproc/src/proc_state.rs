use std::any::Any;
use std::ops::{Deref, DerefMut};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::{fmt, mem};
use std::fmt::{Formatter, Error};

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