use crate::actor::actor_ref::ActorRef;
use crate::actor::state_codes::*;
use crate::errors::*;
use crate::message::TypedMessage;
use async_channel::{unbounded, Receiver, Sender};
use lever::sync::atomics::AtomicBox;
use std::fmt::{self, Debug, Formatter};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Struct that represents a message sender.
#[derive(Clone)]
pub struct MailboxTx<T>
where
    T: TypedMessage,
{
    /// Indicated the transmitter part of the actor's channel
    /// which is using for passing messages.
    tx: Sender<Envelope<T>>,
    /// A field for checks that the message has been delivered to
    /// the specific actor.
    scheduled: Arc<AtomicBool>,
}

impl<T> MailboxTx<T>
where
    T: TypedMessage,
{
    /// Return a new instance of MailboxTx that indicates sender.
    pub(crate) fn new(tx: Sender<Envelope<T>>) -> Self {
        let scheduled = Arc::new(AtomicBool::new(false));
        MailboxTx { tx, scheduled }
    }

    /// Send the message to the actor by the channel.
    pub fn try_send(&self, msg: Envelope<T>) -> BastionResult<()> {
        self.tx
            .try_send(msg)
            .map_err(|e| BError::ChanSend(e.to_string()))
    }
}

/// A struct that holds everything related to messages that can be
/// retrieved from other actors. Each actor holds two queues: one for
/// messages that come from user-defined actors, and another for
/// internal messaging that must be handled separately.
///
/// For each used queue, mailbox always holds the latest requested message
/// by a user, to guarantee that the message won't be lost if something
/// happens wrong.
#[derive(Clone)]
pub struct Mailbox<T>
where
    T: TypedMessage,
{
    /// User guardian sender
    user_tx: MailboxTx<T>,
    /// User guardian receiver
    user_rx: Receiver<Envelope<T>>,
    /// System guardian receiver
    system_rx: Receiver<Envelope<T>>,
    /// The current processing message, received from the
    /// latest call to the user's queue
    last_user_message: Option<Envelope<T>>,
    /// The current processing message, received from the
    /// latest call to the system's queue
    last_system_message: Option<Envelope<T>>,
    /// Mailbox state machine
    state: Arc<AtomicBox<MailboxState>>,
}

// TODO: Add calls with recv with timeout
impl<T> Mailbox<T>
where
    T: TypedMessage,
{
    /// Creates a new mailbox for the actor.
    pub(crate) fn new(system_rx: Receiver<Envelope<T>>) -> Self {
        let (tx, user_rx) = unbounded();
        let user_tx = MailboxTx::new(tx);
        let state = Arc::new(AtomicBox::new(MailboxState::Scheduled));
        let last_user_message = None;
        let last_system_message = None;

        Mailbox {
            user_tx,
            user_rx,
            system_rx,
            last_user_message,
            last_system_message,
            state,
        }
    }

    /// Forced receive message from user queue
    pub async fn recv(&mut self) -> Envelope<T> {
        let message = self
            .user_rx
            .recv()
            .await
            .map_err(|e| BError::ChanRecv(e.to_string()))
            .unwrap();

        self.last_user_message = Some(message);
        self.last_user_message.clone().unwrap()
    }

    /// Try receiving message from user queue
    pub async fn try_recv(&mut self) -> BastionResult<Envelope<T>> {
        if self.last_user_message.is_some() {
            return Err(BError::UnackedMessage);
        }

        match self.user_rx.try_recv() {
            Ok(message) => {
                self.last_user_message = Some(message);
                Ok(self.last_user_message.clone().unwrap())
            }
            Err(e) => Err(BError::ChanRecv(e.to_string())),
        }
    }

    /// Forced receive message from system queue
    pub async fn sys_recv(&mut self) -> Envelope<T> {
        let message = self
            .system_rx
            .recv()
            .await
            .map_err(|e| BError::ChanRecv(e.to_string()))
            .unwrap();

        self.last_system_message = Some(message);
        self.last_system_message.clone().unwrap()
    }

    /// Try receiving message from system queue
    pub async fn try_sys_recv(&mut self) -> BastionResult<Envelope<T>> {
        if self.last_system_message.is_some() {
            return Err(BError::UnackedMessage);
        }

        match self.system_rx.try_recv() {
            Ok(message) => {
                self.last_system_message = Some(message);
                Ok(self.last_system_message.clone().unwrap())
            }
            Err(e) => Err(BError::ChanRecv(e.to_string())),
        }
    }

    /// Returns the last retrieved message from the user channel
    pub async fn get_last_user_message(&self) -> Option<Envelope<T>> {
        self.last_user_message.clone()
    }

    /// Returns the last retrieved message from the system channel
    pub async fn get_last_system_message(&self) -> Option<Envelope<T>> {
        self.last_system_message.clone()
    }

    //
    // Mailbox state machine
    //
    // For more information about the actor's state machine
    // see the actor/state_codes.rs module.
    //

    pub(crate) fn set_scheduled(&self) {
        self.state.replace_with(|_| MailboxState::Scheduled);
    }

    pub(crate) fn is_scheduled(&self) -> bool {
        *self.state.get() == MailboxState::Scheduled
    }

    pub(crate) fn set_sent(&self) {
        self.state.replace_with(|_| MailboxState::Sent);
    }

    pub(crate) fn is_sent(&self) -> bool {
        *self.state.get() == MailboxState::Sent
    }

    pub(crate) fn set_awaiting(&self) {
        self.state.replace_with(|_| MailboxState::Awaiting);
    }

    pub(crate) fn is_awaiting(&self) -> bool {
        *self.state.get() == MailboxState::Awaiting
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

// TODO: Add tests
