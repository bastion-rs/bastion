use crate::children::Message;
use crate::context::BastionId;
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::prelude::*;
use fxhash::FxHashMap;
use std::pin::Pin;
use std::task::{Context, Poll};

pub(super) type Sender = UnboundedSender<BastionMessage>;
pub(super) type Receiver = UnboundedReceiver<BastionMessage>;

pub(super) struct Broadcast {
    id: BastionId,
    sender: Sender,
    recver: Receiver,
    parent: Option<Sender>,
    children: FxHashMap<BastionId, Sender>,
}

#[derive(Debug)]
pub(super) enum BastionMessage {
    PoisonPill,
    Dead { id: BastionId },
    Faulted { id: BastionId },
    Message(Box<dyn Message>),
}

impl Broadcast {
    pub(super) fn new() -> Self {
        let id = BastionId::new();
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

    pub(super) fn with_parent(parent: Sender) -> Self {
        let id = BastionId::new();
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

    pub(super) fn id(&self) -> &BastionId {
        &self.id
    }

    pub(super) fn sender(&self) -> &Sender {
        &self.sender
    }

    pub(super) fn poison_pill_child(&mut self, id: &BastionId) {
        self.send_child(id, BastionMessage::PoisonPill);
        self.remove_child(id);
    }

    pub(super) fn poison_pill_children(&mut self) {
        self.send_children(BastionMessage::PoisonPill);
        self.clear_children();
    }

    pub(super) fn dead(&mut self) {
        self.poison_pill_children();
        self.send_parent(BastionMessage::dead(self.id.clone()));
    }

    pub(super) fn faulted(&mut self) {
        self.poison_pill_children();
        self.send_parent(BastionMessage::faulted(self.id.clone()));
    }

    pub(super) fn new_child(&mut self) -> Self {
        let child = Broadcast::with_parent(self.sender.clone());
        self.children.insert(child.id.clone(), child.sender.clone());

        child
    }

    pub(super) fn remove_child(&mut self, id: &BastionId) -> bool {
        self.children.remove(id).is_some()
    }

    pub(super) fn clear_children(&mut self) {
        self.children.clear();
    }

    pub(super) fn send_parent(&mut self, msg: BastionMessage) {
        // FIXME: Err if None?
        if let Some(parent) = &mut self.parent {
            // FIXME: handle errors
            parent.unbounded_send(msg).ok();
        }
    }

    pub(super) fn send_child(&mut self, id: &BastionId, msg: BastionMessage) {
        // FIXME: Err if None?
        if let Some(child) = self.children.get_mut(id) {
            // FIXME: handle errors
            child.unbounded_send(msg).ok();
        }
    }

    pub(super) fn send_children(&mut self, msg: BastionMessage) {
        for (_, child) in &mut self.children {
            // FIXME: handle errors
            child.unbounded_send(msg.clone()).ok();
        }
    }
}

impl BastionMessage {
    pub(super) fn poison_pill() -> Self {
        BastionMessage::PoisonPill
    }

    pub(super) fn dead(id: BastionId) -> Self {
        BastionMessage::Dead { id }
    }

    pub(super) fn faulted(id: BastionId) -> Self {
        BastionMessage::Faulted { id }
    }

    pub(super) fn msg(msg: Box<dyn Message>) -> Self {
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

    pub(super) fn into_msg(self) -> Option<Box<dyn Message>> {
        if let BastionMessage::Message(msg) = self {
            Some(msg)
        } else {
            None
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
