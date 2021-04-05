use std::fmt::Debug;

use crate::{
    child_ref::SendError,
    message::{Answer, Message},
    prelude::ChildRef,
    system::{STRING_INTERNER, SYSTEM},
};
use anyhow::Result as AnyResult;
use lasso::Spur;

// Copy is fine here because we're working
// with interned strings here
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Distributor(Spur);

impl Distributor {
    pub fn named(name: impl AsRef<str>) -> Self {
        Self(STRING_INTERNER.get_or_intern(name.as_ref()))
    }

    pub fn ask_one(&self, question: impl Message) -> Result<Answer, SendError> {
        SYSTEM.dispatcher().ask(*self, question)
    }

    // todo: create an actual iterator?
    pub fn ask_everyone(&self, question: impl Message + Clone) -> Result<Vec<Answer>, SendError> {
        SYSTEM.dispatcher().ask_everyone(*self, question)
    }

    pub fn tell_one(&self, message: impl Message) -> Result<(), SendError> {
        SYSTEM.dispatcher().tell(*self, message)
    }

    pub fn tell_everyone(&self, message: impl Message + Clone) -> Result<Vec<()>, SendError> {
        SYSTEM.dispatcher().tell_everyone(*self, message)
    }

    pub fn subscribe(&self, child_ref: ChildRef) -> AnyResult<()> {
        let global_dispatcher = SYSTEM.dispatcher();
        global_dispatcher.register_recipient(*self, child_ref)
    }

    pub fn unsubscribe(&self, child_ref: ChildRef) -> AnyResult<()> {
        let global_dispatcher = SYSTEM.dispatcher();
        global_dispatcher.remove_recipient(&vec![*self], child_ref)
    }

    pub(crate) fn interned(&self) -> &Spur {
        &self.0
    }
}
