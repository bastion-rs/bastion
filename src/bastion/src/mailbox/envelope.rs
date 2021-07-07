use std::cell::RefCell;
use std::fmt::{self, Debug, Formatter};

use crate::actor::actor_ref::ActorRef;
use crate::mailbox::message::{Message, MessageType};

/// Struct that represents an incoming message in the actor's mailbox.
#[derive(Clone)]
pub struct Envelope<T>
where
    T: Message,
{
    /// The sending side of a channel. In actor's world
    /// represented is a message sender. Can be used
    /// for acking message when it possible.
    sender: Option<ActorRef>,
    /// An actual data sent by the channel
    message: RefCell<Option<T>>,
    /// Message type that helps to figure out how to deliver message
    /// and how to ack it after the processing.
    message_type: MessageType,
}

impl<T> Envelope<T>
where
    T: Message,
{
    /// Create a message with the given sender and inner data.
    pub fn new(sender: Option<ActorRef>, data: T, message_type: MessageType) -> Self {
        let message = RefCell::new(Some(data));

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

    /// Extracts the message data and returns it to the caller. Each further
    /// method call will return `None`.
    pub fn read(&self) -> Option<T> {
        self.message.replace(None)
    }

    // TODO: Return a boolean flag once operation has finished?
    /// Sends a confirmation to the message sender.
    pub async fn ack(&self) {
        match self.message_type {
            MessageType::Ask => unimplemented!(),
            MessageType::Broadcast => unimplemented!(),
            MessageType::Tell => unimplemented!(),
        }
    }
}

impl<T> Debug for Envelope<T>
where
    T: Message,
{
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("Message")
            .field("message", &self.message)
            .field("message_type", &self.message_type)
            .finish()
    }
}

#[cfg(test)]
mod envelope_tests {
    use crate::mailbox::envelope::Envelope;
    use crate::mailbox::message::{Message, MessageType};

    #[derive(Debug)]
    struct FakeMessage;

    impl FakeMessage {
        pub fn new() -> Self {
            return FakeMessage {};
        }
    }

    #[test]
    fn test_message_read() {
        let message_data = FakeMessage::new();
        let instance = Envelope::new(None, message_data, MessageType::Tell);

        let expected_data = instance.read();
        assert_eq!(expected_data.is_some(), true);

        let another_read_attempt_data = instance.read();
        assert_eq!(another_read_attempt_data.is_none(), true);
    }

    #[test]
    fn test_match_against_ask_message_type() {
        let message_data = FakeMessage::new();
        let instance = Envelope::new(None, message_data, MessageType::Ask);

        assert_eq!(instance.message_type, MessageType::Ask);
    }

    #[test]
    fn test_match_against_broadcast_message_type() {
        let message_data = FakeMessage::new();
        let instance = Envelope::new(None, message_data, MessageType::Broadcast);

        assert_eq!(instance.message_type, MessageType::Broadcast);
    }

    #[test]
    fn test_match_against_tell_message_type() {
        let message_data = FakeMessage::new();
        let instance = Envelope::new(None, message_data, MessageType::Tell);

        assert_eq!(instance.message_type, MessageType::Tell);
    }
}
