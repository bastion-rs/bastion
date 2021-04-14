use std::fmt::{self, Debug, Formatter};

use crate::actor::actor_ref::ActorRef;
use crate::mailbox::message::MessageType;
use crate::mailbox::traits::TypedMessage;

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
