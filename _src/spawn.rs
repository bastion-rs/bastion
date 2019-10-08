use crate::child::{BastionChildren, BastionClosure, Message};

pub trait RuntimeSpawn {
    fn spawn<F, M>(thunk: F, msg: M) -> BastionChildren
    where
        F: BastionClosure,
        M: Message;
}
