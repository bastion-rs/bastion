use crate::context::BastionContext;
use core::fmt;
use core::fmt::{Debug, Formatter};
use crossbeam_channel::{Receiver, Sender};
use std::any::Any;
use uuid::Uuid;

pub trait Shell: Send + Sync + objekt::Clone + Any + 'static {}
impl<T> Shell for T where T: Send + Sync + objekt::Clone + Any + 'static {}

pub trait Message: Shell + Debug {
    fn as_any(&self) -> &dyn Any;
}
impl<T> Message for T
where
    T: Shell + Debug,
{
    #[inline(always)]
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub trait BastionClosure: Fn(BastionContext, Box<dyn Message>) + Shell {}
impl<T> BastionClosure for T where T: Fn(BastionContext, Box<dyn Message>) + Shell {}

pub struct BastionChildren {
    pub id: String,
    pub redundancy: i32,
    pub tx: Option<Sender<Box<dyn Message>>>,
    pub rx: Option<Receiver<Box<dyn Message>>>,
    pub thunk: Box<dyn BastionClosure>,
    pub msg: Box<dyn Message>,
}

impl PartialEq for BastionChildren {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.redundancy == other.redundancy
    }
}

impl Debug for BastionChildren {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        // TODO: add thunk ref address here
        write!(
            f,
            "(ID :: {:?}, Redundancy :: {:?})",
            self.id, self.redundancy
        )
    }
}

impl Default for BastionChildren {
    fn default() -> Self {
        let e = Box::new(|_: BastionContext, _: Box<dyn Message>| {});
        let m = Box::new("default".to_string());

        BastionChildren {
            id: Uuid::new_v4().to_string(),
            redundancy: 1_i32,
            tx: None,
            rx: None,
            thunk: e,
            msg: m,
        }
    }
}

impl Clone for BastionChildren {
    fn clone(&self) -> Self {
        BastionChildren {
            id: self.id.to_string(),
            tx: self.tx.clone(),
            rx: self.rx.clone(),
            thunk: objekt::clone_box(&*self.thunk),
            redundancy: self.redundancy,
            msg: objekt::clone_box(&*self.msg),
        }
    }
}
