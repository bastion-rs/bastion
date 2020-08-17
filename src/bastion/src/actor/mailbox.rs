use crate::message::*;
use async_channel::{Receiver, Sender};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[derive(Clone)]
pub struct Mailbox<T>
where
    T: TypedMessage,
{
    inner: Arc<MailboxInner<T>>,
}

pub struct MailboxInner<T>
where
    T: TypedMessage,
{
    /// User guardian receiver
    user_rx: Receiver<T>,
    /// System guardian receiver
    sys_rx: Receiver<T>,
}

#[derive(Clone)]
pub struct MailboxTx<T>
where
    T: TypedMessage,
{
    tx: Sender<T>,
    scheduled: Arc<AtomicBool>,
}
