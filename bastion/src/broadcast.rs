use crate::bastion::SYSTEM;
use crate::children::{Children, ChildrenRef, Message};
use crate::context::BastionId;
use crate::supervisor::{SupervisionStrategy, Supervisor, SupervisorRef};
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::prelude::*;
use fxhash::FxHashMap;
use std::pin::Pin;
use std::task::{Context, Poll};

pub(super) type Sender = UnboundedSender<BastionMessage>;
pub(super) type Receiver = UnboundedReceiver<BastionMessage>;

#[derive(Debug)]
pub(super) struct Broadcast {
    id: BastionId,
    sender: Sender,
    recver: Receiver,
    parent: Parent,
    children: FxHashMap<BastionId, Sender>,
}

#[derive(Debug)]
pub(super) enum Parent {
    None,
    System,
    Supervisor(SupervisorRef),
    Children(ChildrenRef),
}

#[derive(Debug)]
pub(super) enum BastionMessage {
    Start,
    Stop,
    PoisonPill,
    Deploy(Deployment),
    Prune { id: BastionId },
    SuperviseWith(SupervisionStrategy),
    Message(Box<dyn Message>),
    Dead { id: BastionId },
    Faulted { id: BastionId },
}

#[derive(Debug)]
pub(super) enum Deployment {
    Supervisor(Supervisor),
    Children(Children),
}

impl Broadcast {
    pub(super) fn new(parent: Parent) -> Self {
        let id = BastionId::new();
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

    pub(super) fn with_id(parent: Parent, id: BastionId) -> Self {
        let mut bcast = Broadcast::new(parent);
        bcast.id = id;

        bcast
    }

    pub(super) fn id(&self) -> &BastionId {
        &self.id
    }

    pub(super) fn sender(&self) -> &Sender {
        &self.sender
    }

    pub(super) fn register(&mut self, child: &Self) {
        self.children.insert(child.id.clone(), child.sender.clone());
    }

    pub(super) fn unregister(&mut self, id: &BastionId) {
        self.children.remove(id);
    }

    pub(super) fn clear_children(&mut self) {
        self.children.clear();
    }

    /*pub(super) fn start_child(&mut self, id: &BastionId) {
        self.send_child(id, BastionMessage::Start);
    }

    pub(super) fn start_children(&mut self) {
        self.send_children(BastionMessage::Start);
    }*/

    pub(super) fn stop_child(&mut self, id: &BastionId) {
        self.send_child(id, BastionMessage::Stop);
        self.unregister(id);
    }

    pub(super) fn stop_children(&mut self) {
        self.send_children(BastionMessage::Stop);
        self.clear_children();
    }

    pub(super) fn kill_child(&mut self, id: &BastionId) {
        self.send_child(id, BastionMessage::PoisonPill);
        self.unregister(id);
    }

    pub(super) fn kill_children(&mut self) {
        self.send_children(BastionMessage::PoisonPill);
        self.clear_children();
    }

    pub(super) fn dead(&mut self) {
        self.stop_children();
        self.send_parent(BastionMessage::dead(self.id.clone()));
    }

    pub(super) fn faulted(&mut self) {
        self.kill_children();
        self.send_parent(BastionMessage::faulted(self.id.clone()));
    }

    pub(super) fn send_parent(&self, msg: BastionMessage) {
        self.parent.send(msg);
    }

    pub(super) fn send_child(&self, id: &BastionId, msg: BastionMessage) {
        // FIXME: Err if None?
        if let Some(child) = self.children.get(id) {
            // FIXME: handle errors
            child.unbounded_send(msg).ok();
        }
    }

    pub(super) fn send_children(&self, msg: BastionMessage) {
        for (_, child) in &self.children {
            // FIXME: handle errors
            child.unbounded_send(msg.clone()).ok();
        }
    }

    pub(super) fn send_self(&self, msg: BastionMessage) {
        // FIXME: handle errors
        self.sender.unbounded_send(msg).ok();
    }
}

impl Parent {
    pub(super) fn none() -> Self {
        Parent::None
    }

    pub(super) fn system() -> Self {
        Parent::System
    }

    pub(super) fn supervisor(supervisor: SupervisorRef) -> Self {
        Parent::Supervisor(supervisor)
    }

    pub(super) fn children(children: ChildrenRef) -> Self {
        Parent::Children(children)
    }

    fn send(&self, msg: BastionMessage) {
        match self {
            // FIXME
            Parent::None => unimplemented!(),
            Parent::System => {
                // FIXME: Err(Error)
                SYSTEM.unbounded_send(msg).ok();
            }
            Parent::Supervisor(supervisor) => {
                supervisor.send(msg);
            }
            Parent::Children(children) => {
                children.send(msg);
            }
        }
    }
}

impl BastionMessage {
    pub(super) fn start() -> Self {
        BastionMessage::Start
    }

    pub(super) fn stop() -> Self {
        BastionMessage::Stop
    }

    pub(super) fn poison_pill() -> Self {
        BastionMessage::PoisonPill
    }

    pub(super) fn deploy_supervisor(supervisor: Supervisor) -> Self {
        let deployment = Deployment::Supervisor(supervisor);

        BastionMessage::Deploy(deployment)
    }

    pub(super) fn deploy_children(children: Children) -> Self {
        let deployment = Deployment::Children(children);

        BastionMessage::Deploy(deployment)
    }

    pub(super) fn prune(id: BastionId) -> Self {
        BastionMessage::Prune { id }
    }

    pub(super) fn supervise_with(strategy: SupervisionStrategy) -> Self {
        BastionMessage::SuperviseWith(strategy)
    }

    pub(super) fn message(msg: Box<dyn Message>) -> Self {
        BastionMessage::Message(msg)
    }

    pub(super) fn dead(id: BastionId) -> Self {
        BastionMessage::Dead { id }
    }

    pub(super) fn faulted(id: BastionId) -> Self {
        BastionMessage::Faulted { id }
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
        Pin::new(&mut self.get_mut().recver).poll_next(ctx)
    }
}

impl Clone for BastionMessage {
    fn clone(&self) -> Self {
        match self {
            BastionMessage::Start => BastionMessage::start(),
            BastionMessage::Stop => BastionMessage::stop(),
            BastionMessage::PoisonPill => BastionMessage::poison_pill(),
            // FIXME
            BastionMessage::Deploy(_) => unimplemented!(),
            BastionMessage::Prune { id } => BastionMessage::prune(id.clone()),
            BastionMessage::SuperviseWith(strategy) => {
                BastionMessage::supervise_with(strategy.clone())
            }
            BastionMessage::Message(msg) => {
                let msg = objekt::clone_box(&**msg);
                BastionMessage::message(msg)
            }
            BastionMessage::Dead { id } => BastionMessage::dead(id.clone()),
            BastionMessage::Faulted { id } => BastionMessage::faulted(id.clone()),
        }
    }
}
