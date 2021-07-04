pub mod envelope;
pub mod message;
pub(crate) mod state;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use async_channel::{unbounded, Receiver, Sender};

use crate::error::{BastionError, Result};
use crate::mailbox::envelope::Envelope;
use crate::mailbox::message::Message;
use crate::mailbox::state::MailboxState;

/// Struct that represents a message sender.
#[derive(Clone)]
pub struct MailboxTx<T>
where
    T: Message,
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
    T: Message,
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
    T: Message,
{
    /// Actor guardian sender
    actor_tx: MailboxTx<T>,
    /// Actor guardian receiver
    actor_rx: Receiver<Envelope<T>>,
    /// System guardian receiver
    system_rx: Receiver<Envelope<T>>,
    /// Mailbox state machine
    state: Arc<MailboxState>,
}

// TODO: Add calls with recv with timeout
impl<T> Mailbox<T>
where
    T: Message,
{
    /// Creates a new mailbox for the actor.
    pub(crate) fn new(system_rx: Receiver<Envelope<T>>) -> Self {
        let (tx, actor_rx) = unbounded();
        let actor_tx = MailboxTx::new(tx);
        let state = Arc::new(MailboxState::new());

        Mailbox {
            actor_tx,
            actor_rx,
            system_rx,
            state,
        }
    }

    /// Forced receive message from the actor's queue.
    pub async fn recv(&mut self) -> Envelope<T> {
        self.actor_rx
            .recv()
            .await
            .map_err(|e| BastionError::ChanRecv(e.to_string()))
            .unwrap()
    }

    /// Try receiving message from the actor's queue.
    pub async fn try_recv(&mut self) -> Result<Envelope<T>> {
        self.actor_rx
            .try_recv()
            .map_err(|e| BastionError::ChanRecv(e.to_string()))
    }

    /// Forced receive message from the internal system queue.
    pub async fn sys_recv(&mut self) -> Envelope<T> {
        self.system_rx
            .recv()
            .await
            .map_err(|e| BastionError::ChanRecv(e.to_string()))
            .unwrap()
    }

    /// Try receiving message from the internal system queue.
    pub async fn try_sys_recv(&mut self) -> Result<Envelope<T>> {
        self.system_rx
            .try_recv()
            .map_err(|e| BastionError::ChanRecv(e.to_string()))
    }
}
