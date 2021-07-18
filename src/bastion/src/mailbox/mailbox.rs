use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use flume::{bounded, unbounded, Receiver, Sender, TryRecvError};

use crate::error::{BastionError, Result};
use crate::mailbox::envelope::Envelope;
use crate::mailbox::message::Message;
use crate::mailbox::state::MailboxState;

/// An enum that defines the mailbox boundaries.
#[derive(Debug, Eq, PartialEq)]
pub enum MailboxSize {
    /// Store only max N messages. Any other incoming messages
    /// that can't be put in the mailbox will be ignored.
    Limited(usize),
    /// No limits for messages. Default value.
    Unlimited,
}

/// Struct that represents a message sender.
#[derive(Clone)]
pub struct MailboxTx {
    /// Indicated the transmitter part of the actor's channel
    /// which is using for passing messages.
    tx: Sender<Envelope>,
    /// A field for checks that the message has been delivered to
    /// the specific actor.
    scheduled: Arc<AtomicBool>,
}

impl MailboxTx {
    /// Return a new instance of MailboxTx that indicates sender.
    pub(crate) fn new(tx: Sender<Envelope>) -> Self {
        let scheduled = Arc::new(AtomicBool::new(false));
        MailboxTx { tx, scheduled }
    }

    /// Send the message to the actor by the channel.
    pub fn try_send(&self, msg: Envelope) -> Result<()> {
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
pub struct Mailbox {
    /// Actor guardian sender
    actor_tx: MailboxTx,
    /// Actor guardian receiver
    actor_rx: Receiver<Envelope>,
    /// System guardian receiver
    system_rx: Receiver<Envelope>,
    /// Mailbox state machine
    state: Arc<MailboxState>,
}

// TODO: Add calls with recv with timeout
impl Mailbox {
    /// Creates a new mailbox for the actor.
    pub(crate) fn new(mailbox_size: MailboxSize, system_rx: Receiver<Envelope>) -> Self {
        let (tx, actor_rx) = match mailbox_size {
            MailboxSize::Limited(limit) => bounded(limit),
            MailboxSize::Unlimited => unbounded(),
        };
        let actor_tx = MailboxTx::new(tx);
        let state = Arc::new(MailboxState::new());

        Mailbox {
            actor_tx,
            actor_rx,
            system_rx,
            state,
        }
    }

    /// Returns an actor sender to the caller.
    pub(crate) fn get_actor_rx(&self) -> Sender<Envelope> {
        self.actor_tx.tx.clone()
    }

    /// Forced receive message from the actor's queue.
    pub async fn recv(&mut self) -> Envelope {
        self.actor_rx
            .recv_async()
            .await
            .map_err(|e| BastionError::ChanRecv(e.to_string()))
            .unwrap()
    }

    /// Try receiving message from the actor's queue.
    pub async fn try_recv(&mut self) -> Result<Envelope> {
        self.actor_rx.try_recv().map_err(|e| match e {
            TryRecvError::Empty => BastionError::EmptyChannel,
            _ => BastionError::ChanRecv(e.to_string()),
        })
    }

    /// Forced receive message from the internal system queue.
    pub async fn sys_recv(&mut self) -> Envelope {
        self.system_rx
            .recv_async()
            .await
            .map_err(|e| BastionError::ChanRecv(e.to_string()))
            .unwrap()
    }

    /// Try receiving message from the internal system queue.
    pub async fn try_sys_recv(&mut self) -> Result<Envelope> {
        self.system_rx.try_recv().map_err(|e| match e {
            TryRecvError::Empty => BastionError::EmptyChannel,
            _ => BastionError::ChanRecv(e.to_string()),
        })
    }
}

#[cfg(test)]
mod mailbox_tests {
    use flume::unbounded;
    use tokio_test::block_on;

    use crate::error::Result;
    use crate::mailbox::envelope::Envelope;
    use crate::mailbox::mailbox::{Mailbox, MailboxSize};
    use crate::mailbox::message::{Message, MessageType};

    #[test]
    fn test_recv() {
        let (system_tx, system_rx) = unbounded();
        let mut instance = Mailbox::new(MailboxSize::Unlimited, system_rx);
        let envelope = Envelope::new(None, Box::new(1), MessageType::Tell);

        tokio_test::block_on(instance.actor_tx.tx.send_async(envelope));
        let incoming_env = tokio_test::block_on(instance.recv());
        let actor_ref = incoming_env.sender();
        let message = tokio_test::block_on(incoming_env.read());
        let message_type = incoming_env.message_type();

        assert_eq!(actor_ref.is_none(), true);
        assert_eq!(message.is_some(), true);
        assert_eq!(message_type, MessageType::Tell);
    }

    #[test]
    fn test_try_recv() {
        let (system_tx, system_rx) = unbounded();
        let mut instance = Mailbox::new(MailboxSize::Unlimited, system_rx);
        let envelope = Envelope::new(None, Box::new(1), MessageType::Tell);

        tokio_test::block_on(instance.actor_tx.tx.send_async(envelope));
        let incoming_env = tokio_test::block_on(instance.try_recv()).unwrap();
        let actor_ref = incoming_env.sender();
        let message = tokio_test::block_on(incoming_env.read());
        let message_type = incoming_env.message_type();

        assert_eq!(actor_ref.is_none(), true);
        assert_eq!(message.is_some(), true);
        assert_eq!(message_type, MessageType::Tell);
    }

    #[test]
    fn test_try_recv_returns_error_on_empty_channel() {
        let (system_tx, system_rx) = unbounded();
        let mut instance = Mailbox::new(MailboxSize::Unlimited, system_rx);

        let result = tokio_test::block_on(instance.try_sys_recv()).ok();
        assert_eq!(result.is_none(), true);
    }

    #[test]
    fn test_sys_recv() {
        let (system_tx, system_rx) = unbounded();
        let mut instance = Mailbox::new(MailboxSize::Unlimited, system_rx);
        let envelope = Envelope::new(None, Box::new(1), MessageType::Tell);

        tokio_test::block_on(system_tx.send_async(envelope));
        let incoming_env = tokio_test::block_on(instance.sys_recv());
        let actor_ref = incoming_env.sender();
        let message = tokio_test::block_on(incoming_env.read());
        let message_type = incoming_env.message_type();

        assert_eq!(actor_ref.is_none(), true);
        assert_eq!(message.is_some(), true);
        assert_eq!(message_type, MessageType::Tell);
    }

    #[test]
    fn test_try_sys_recv() {
        let (system_tx, system_rx) = unbounded();
        let mut instance = Mailbox::new(MailboxSize::Unlimited, system_rx);
        let envelope = Envelope::new(None, Box::new(1), MessageType::Tell);

        tokio_test::block_on(system_tx.send_async(envelope));
        let incoming_env = tokio_test::block_on(instance.try_sys_recv()).unwrap();
        let actor_ref = incoming_env.sender();
        let message = tokio_test::block_on(incoming_env.read());
        let message_type = incoming_env.message_type();

        assert_eq!(actor_ref.is_none(), true);
        assert_eq!(message.is_some(), true);
        assert_eq!(message_type, MessageType::Tell);
    }

    #[test]
    fn test_try_sys_recv_returns_error_on_empty_channel() {
        let (system_tx, system_rx) = unbounded();
        let mut instance = Mailbox::new(MailboxSize::Unlimited, system_rx);

        let result = tokio_test::block_on(instance.try_sys_recv()).ok();
        assert_eq!(result.is_none(), true);
    }
}
