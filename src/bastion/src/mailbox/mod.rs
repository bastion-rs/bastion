mod envelope;
mod state;

pub mod message;
pub mod traits;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use async_channel::{unbounded, Receiver, Sender};

use crate::error::{BastionError, Result};
use crate::mailbox::envelope::Envelope;
use crate::mailbox::state::MailboxState;
use crate::mailbox::traits::TypedMessage;

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
    pub fn try_send(&self, msg: Envelope<T>) -> Result<()> {
        self.tx
            .try_send(msg)
            .map_err(|e| BastionError::ChanSend(e.to_string()))
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
    state: Arc<MailboxState>,
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
        let last_user_message = None;
        let last_system_message = None;
        let state = Arc::new(MailboxState::new());

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
            .map_err(|e| BastionError::ChanRecv(e.to_string()))
            .unwrap();

        self.last_user_message = Some(message);
        self.last_user_message.clone().unwrap()
    }

    /// Try receiving message from user queue
    pub async fn try_recv(&mut self) -> Result<Envelope<T>> {
        if self.last_user_message.is_some() {
            return Err(BastionError::UnackedMessage);
        }

        match self.user_rx.try_recv() {
            Ok(message) => {
                self.last_user_message = Some(message);
                Ok(self.last_user_message.clone().unwrap())
            }
            Err(e) => Err(BastionError::ChanRecv(e.to_string())),
        }
    }

    /// Forced receive message from system queue
    pub async fn sys_recv(&mut self) -> Envelope<T> {
        let message = self
            .system_rx
            .recv()
            .await
            .map_err(|e| BastionError::ChanRecv(e.to_string()))
            .unwrap();

        self.last_system_message = Some(message);
        self.last_system_message.clone().unwrap()
    }

    /// Try receiving message from system queue
    pub async fn try_sys_recv(&mut self) -> Result<Envelope<T>> {
        if self.last_system_message.is_some() {
            return Err(BastionError::UnackedMessage);
        }

        match self.system_rx.try_recv() {
            Ok(message) => {
                self.last_system_message = Some(message);
                Ok(self.last_system_message.clone().unwrap())
            }
            Err(e) => Err(BastionError::ChanRecv(e.to_string())),
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
}
