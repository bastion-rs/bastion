use crate::children::Message;
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::prelude::*;
use fxhash::FxHashMap;
use std::pin::Pin;
use std::task::{Context, Poll};
use uuid::Uuid;

type Sender = UnboundedSender<BastionMessage>;
type Receiver = UnboundedReceiver<BastionMessage>;

pub(super) struct Broadcast {
    id: Uuid,
    sender: Sender,
    recver: Receiver,
    parent: Option<Sender>,
    children: FxHashMap<Uuid, Sender>,
}

#[derive(Debug)]
pub(super) enum BastionMessage {
    PoisonPill,
    Dead {
        id: Uuid,
    },
    Faulted {
        id: Uuid,
    },
    Message(Box<dyn Message>),
}

impl Broadcast {
    pub(super) fn new(id: Uuid) -> Self {
        let parent = None;
        let (sender, recver) = mpsc::unbounded();
        let children = FxHashMap::default();

        Broadcast {
            id,
            parent,
            sender,
            recver,
            children,
        }
    }

    pub(super) fn with_parent(id: Uuid, parent: Sender) -> Self {
        let parent = Some(parent);
        let (sender, recver) = mpsc::unbounded();
        let children = FxHashMap::default();

        Broadcast {
            id,
            parent,
            sender,
            recver,
            children,
        }
    }

    pub(super) fn id(&self) -> &Uuid {
        &self.id
    }

    pub(super) fn new_child(&mut self, id: Uuid) -> Self {
        let child = Broadcast::with_parent(id, self.sender.clone());
        self.children.insert(child.id.clone(), child.sender.clone());

        child
    }

    pub(super) fn remove_child(&mut self, id: &Uuid) -> bool {
        self.children.remove(id).is_some()
    }

    pub(super) fn clear_children(&mut self) {
        self.children.clear();
    }

    pub(super) fn send_parent(&mut self, msg: BastionMessage) {
        // FIXME: Err if None?
        if let Some(parent) = &mut self.parent {
            // FIXME: handle errors
            parent.unbounded_send(msg);
        }
    }

    pub(super) fn send_child(&mut self, id: &Uuid, msg: BastionMessage) {
        // FIXME: Err if None?
        if let Some(child) = self.children.get_mut(id) {
            // FIXME: handle errors
            child.unbounded_send(msg);
        }
    }

    pub(super) fn send_children(&mut self, msg: BastionMessage) {
        for (_, child) in &mut self.children {
            // FIXME: handle errors
            child.unbounded_send(msg.clone());
        }
    }
}

impl BastionMessage {
    pub(super) fn poison_pill() -> BastionMessage {
        BastionMessage::PoisonPill
    }

    pub(super) fn dead(id: Uuid) -> BastionMessage {
        BastionMessage::Dead { id }
    }

    pub(super) fn faulted(id: Uuid) -> BastionMessage {
        BastionMessage::Faulted { id }
    }

    pub(super) fn msg(msg: Box<dyn Message>) -> BastionMessage {
        BastionMessage::Message(msg)
    }

    pub(super) fn is_poison_pill(&self) -> bool {
        if let BastionMessage::PoisonPill = self {
            true
        } else {
            false
        }
    }

    pub(super) fn is_dead(&self) -> bool {
        if let BastionMessage::Dead { .. } = self {
            true
        } else {
            false
        }
    }

    pub(super) fn is_faulted(&self) -> bool {
        if let BastionMessage::Faulted { .. } = self {
            true
        } else {
            false
        }
    }

    pub(super) fn is_msg(&self) -> bool {
        if let BastionMessage::Message(_) = self {
            true
        } else {
            false
        }
    }
}

impl Stream for Broadcast {
    type Item = BastionMessage;

    fn poll_next(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Option<Self::Item>> {
        let bcast = self.get_mut();

        Pin::new(&mut bcast.recver).poll_next(ctx)
    }
}

impl Clone for BastionMessage {
    fn clone(&self) -> Self {
        match self {
            BastionMessage::PoisonPill => BastionMessage::poison_pill(),
            BastionMessage::Dead { id } => BastionMessage::dead(id.clone()),
            BastionMessage::Faulted { id } => BastionMessage::faulted(id.clone()),
            BastionMessage::Message(msg) => BastionMessage::msg(objekt::clone_box(&**msg)),
        }
    }
}