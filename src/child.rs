//!
//! Child management, message generics and communication reside in this module.
//!
use crate::context::BastionContext;
use core::fmt;
use core::fmt::{Debug, Formatter};
use crossbeam_channel::{Receiver, Sender};
use std::any::Any;
use uuid::Uuid;

///
/// Generic trait for all possible usable structs in system-wide
pub trait Shell: Send + Sync + objekt::Clone + Any + 'static {}
impl<T> Shell for T where T: Send + Sync + objekt::Clone + Any + 'static {}

///
/// Message trait as bare minimum message contract.
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

/// Base minimum trait contract of process body closure.
pub trait BastionClosure: Fn(BastionContext, Box<dyn Message>) + Shell {}
impl<T> BastionClosure for T where T: Fn(BastionContext, Box<dyn Message>) + Shell {}

///
/// Children representation from the system's perspective.
pub struct BastionChildren {
    /// ID of the child. Assembled as
    ///
    /// Assembled as `Supervisor URN + child id`
    pub id: String,

    /// Redundancy count given by the spawn function of supervisor to this children group.
    pub redundancy: i32,

    /// Children's communication channel's sender endpoint
    pub tx: Option<Sender<Box<dyn Message>>>,

    /// Children's communication channel's receiver endpoint
    pub rx: Option<Receiver<Box<dyn Message>>>,

    /// Main children process body
    pub thunk: Box<dyn BastionClosure>,

    /// Initial message passed down to the children
    pub msg: Box<dyn Message>,
}

impl PartialEq for BastionChildren {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.redundancy == other.redundancy
    }
}

impl Debug for BastionChildren {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        f.debug_struct("BastionChildren")
            .field("id", &self.id)
            .field("redundancy", &self.redundancy)
            .field("thunk_ptr", &format_args!("{:p}", &self.thunk as *const _))
            .finish()
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
