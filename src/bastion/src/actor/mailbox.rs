use crate::actor::actor_ref::ActorRef;
use crate::message::TypedMessage;
use async_channel::{Receiver, Sender};
use std::collections::VecDeque;
use std::fmt::{self, Debug, Formatter};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[derive(Clone)]
pub struct Mailbox<T>
where
    T: TypedMessage,
{
    receiver: Arc<MailboxInner<T>>,
    envelopes: VecDeque<Arc<Envelope<T>>>,
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

/// Struct that represents a message sender.
#[derive(Clone)]
pub struct MailboxTx<T>
where
    T: TypedMessage,
{
    tx: Sender<T>,
    scheduled: Arc<AtomicBool>,
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
    sender: Arc<ActorRef>,
    /// An actual data sent by the channel
    message: Arc<T>,
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

impl<T> Mailbox<T>
where
    T: TypedMessage,
{
    /// Creates a new mailbox for the actor.
    pub fn new(receiver: Arc<MailboxInner<T>>) -> Self {
        Mailbox {
            receiver,
            envelopes: VecDeque::new(),
        }
    }

    /// Adds a new envelope into the actor's mailbox.
    pub(crate) fn push_envelope(&mut self, envelope: Envelope<T>) {
        self.envelopes.push_back(Arc::new(envelope));
    }

    /// Returns the latest envelope from the mailbox.
    pub(crate) fn pop_envelope(&self) -> Option<Arc<Envelope<T>>> {
        match self.envelopes.get(0) {
            Some(envelope) => Some(envelope.clone()),
            None => None,
        }
    }

    /// Drops the latest envelope from the mailbox, when it's processed.
    pub(crate) fn ack_envelope(&mut self) {
        match self.envelopes.pop_front() {
            Some(_) => {}
            None => {}
        }
    }
}

impl<T> Envelope<T>
where
    T: TypedMessage,
{
    /// Create a message with the given sender and inner data.
    pub fn new(sender: Arc<ActorRef>, data: T, message_type: MessageType) -> Self {
        Envelope {
            sender,
            message: Arc::new(data),
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
