use crate::actor::actor_ref::ActorRef;
use crate::message::TypedMessage;
use async_channel::{Receiver, Sender};
use std::collections::VecDeque;
use std::fmt::{self, Debug, Formatter};
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::Arc;
use lever::sync::atomics::AtomicBox;
use crate::actor::state_codes::*;
use crate::errors::*;

pub struct MailboxInner<T>
where
    T: TypedMessage,
{
    /// User guardian receiver
    user_rx: Receiver<Envelope<T>>,
    /// System guardian receiver
    sys_rx: Receiver<Envelope<T>>,
    /// Mailbox state machine
    state: Arc<AtomicBox<MailboxState>>
}

impl<T> MailboxInner<T>
where
    T: TypedMessage
{
    /// User messages receiver channel
    fn user_rx(&self) -> &Receiver<Envelope<T>> {
        &self.user_rx
    }

    /// System messages receiver channel
    fn sys_rx(&self) -> &Receiver<Envelope<T>> {
        &self.sys_rx
    }

    /// Mailbox state
    fn state(&self) -> &Arc<AtomicBox<MailboxState>> {
        &self.state
    }
}

/// Struct that represents a message sender.
#[derive(Clone)]
pub struct MailboxTx<T>
where
    T: TypedMessage,
{
    tx: Sender<Envelope<T>>,
    scheduled: Arc<AtomicBool>,
}

unsafe impl<T> Send for MailboxTx<T> where T: TypedMessage {}
unsafe impl<T> Sync for MailboxTx<T> where T: TypedMessage {}

impl<T> MailboxTx<T>
where
    T: TypedMessage
{
    pub fn try_send(&self, msg: Envelope<T>) -> Result<()> {
        self.tx.try_send(msg)
            .map_err(|e| BError::ChanSend(e.to_string()))
    }
}

#[derive(Clone)]
pub struct Mailbox<T>
where
    T: TypedMessage,
{
    inner: Arc<MailboxInner<T>>
}

impl<T> Mailbox<T>
where
    T: TypedMessage,
{
    /// Creates a new mailbox for the actor.
    pub fn new(inner: Arc<MailboxInner<T>>) -> Self {
        Mailbox { inner }
    }

    /// Forced receive message from user queue
    pub async fn recv(&self) -> Envelope<T> {
        self.inner.user_rx().recv().await
            .map_err(|e| BError::ChanRecv(e.to_string()))
            .unwrap()
    }

    /// Try receiving message from user queue
    pub async fn try_recv(&self) -> Result<Envelope<T>> {
        self.inner.user_rx().try_recv()
            .map_err(|e| BError::ChanRecv(e.to_string()))
    }

    /// Forced receive message from user queue
    pub async fn sys_recv(&self) -> Envelope<T> {
        self.inner.sys_rx().recv().await
            .map_err(|e| BError::ChanRecv(e.to_string()))
            .unwrap()
    }

    /// Try receiving message from user queue
    pub async fn try_sys_recv(&self) -> Result<Envelope<T>> {
        self.inner.sys_rx().try_recv()
            .map_err(|e| BError::ChanRecv(e.to_string()))
    }

    //////////////////////
    ///// State machine
    //////////////////////

    pub(crate) fn set_scheduled(&self) {
        self.inner.state().replace_with(|_| MailboxState::Scheduled);
    }

    pub(crate) fn is_scheduled(&self) -> bool {
        *self.inner.state().get() == MailboxState::Scheduled
    }

    pub(crate) fn set_sent(&self) {
        self.inner.state().replace_with(|_| MailboxState::Sent);
    }

    pub(crate) fn is_sent(&self) -> bool {
        *self.inner.state().get() == MailboxState::Sent
    }

    pub(crate) fn set_awaiting(&self) {
        self.inner.state().replace_with(|_| MailboxState::Awaiting);
    }

    pub(crate) fn is_awaiting(&self) -> bool {
        *self.inner.state().get() == MailboxState::Awaiting
    }
}

/// Struct that represents an incoming message in the actor's mailbox.
#[derive(Clone)]
pub struct Envelope<T>
where
    T: TypedMessage,
{
    /// The sending side of a channel. In actor's world
    /// represented is a message sender. Can be used
    /// for acking message when it possible.
    sender: Option<ActorRef>,
    /// An actual data sent by the channel
    message: T,
    /// Message type that helps to figure out how to deliver message
    /// and how to ack it after the processing.
    message_type: MessageType,
}

/// Enum that provides information what type of the message
/// being sent through the channel.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MessageType {
    /// A message type that requires sending a confirmation to the
    /// sender after begin the processing stage.
    Ack,
    /// A message that were broadcasted (e.g. via system dispatchers). This
    /// message type doesn't require to be acked from the receiver's side.
    Broadcast,
    /// A message was sent directly and doesn't require confirmation for the
    /// delivery and being processed.
    Tell,
}

impl<T> Envelope<T>
where
    T: TypedMessage,
{
    /// Create a message with the given sender and inner data.
    pub fn new(sender: Option<ActorRef>, message: T, message_type: MessageType) -> Self {
        Envelope {
            sender,
            message,
            message_type,
        }
    }

    /// Returns a message type. Can be use for pattern matching and filtering
    /// incoming message from other actors.
    pub fn message_type(&self) -> MessageType {
        self.message_type.clone()
    }

    /// Sends a confirmation to the message sender.
    pub(crate) async fn ack(&self) {
        match self.message_type {
            MessageType::Ack => unimplemented!(),
            MessageType::Broadcast => unimplemented!(),
            MessageType::Tell => unimplemented!(),
        }
    }
}

impl<T> Debug for Envelope<T>
where
    T: TypedMessage,
{
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("Message")
            .field("message", &self.message)
            .field("message_type", &self.message_type)
            .finish()
    }
}
