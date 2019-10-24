use crate::children::{Children, ChildrenRef, Message};
use crate::context::BastionId;
use crate::supervisor::{SupervisionStrategy, Supervisor, SupervisorRef};
use crate::system::SYSTEM;
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
    Kill,
    Deploy(Deployment),
    Prune { id: BastionId },
    SuperviseWith(SupervisionStrategy),
    Message(Box<dyn Message>),
    Stopped { id: BastionId },
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

    pub(super) fn stop_child(&mut self, id: &BastionId) {
        let msg = BastionMessage::stop();
        self.send_child(id, msg);

        self.unregister(id);
    }

    pub(super) fn stop_children(&mut self) {
        let msg = BastionMessage::stop();
        self.send_children(msg);

        self.clear_children();
    }

    pub(super) fn kill_child(&mut self, id: &BastionId) {
        let msg = BastionMessage::kill();
        self.send_child(id, msg);

        self.unregister(id);
    }

    pub(super) fn kill_children(&mut self) {
        let msg = BastionMessage::kill();
        self.send_children(msg);

        self.clear_children();
    }

    pub(super) fn stopped(&mut self) {
        self.stop_children();

        let msg = BastionMessage::stopped(self.id.clone());
        // FIXME: Err(msg)
        self.send_parent(msg).ok();
    }

    pub(super) fn faulted(&mut self) {
        self.kill_children();

        let msg = BastionMessage::faulted(self.id.clone());
        // FIXME: Err(msg)
        self.send_parent(msg).ok();
    }

    pub(super) fn send_parent(&self, msg: BastionMessage) -> Result<(), BastionMessage> {
        self.parent.send(msg)
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

    fn send(&self, msg: BastionMessage) -> Result<(), BastionMessage> {
        match self {
            // FIXME
            Parent::None => unimplemented!(),
            Parent::System => SYSTEM.unbounded_send(msg).map_err(|err| err.into_inner()),
            Parent::Supervisor(supervisor) => supervisor.send(msg),
            Parent::Children(children) => children.send(msg),
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

    pub(super) fn kill() -> Self {
        BastionMessage::Kill
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

    pub(super) fn stopped(id: BastionId) -> Self {
        BastionMessage::Stopped { id }
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
            BastionMessage::Kill => BastionMessage::kill(),
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
            BastionMessage::Stopped { id } => BastionMessage::stopped(id.clone()),
            BastionMessage::Faulted { id } => BastionMessage::faulted(id.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BastionMessage, Broadcast, Parent};
    use futures::poll;
    use futures::prelude::*;
    use std::task::Poll;
    use tokio::runtime::Runtime;

    #[test]
    fn send_children() {
        let mut parent = Broadcast::new(Parent::none());

        let mut children = vec![];
        for _ in 0..4 {
            let child = Broadcast::new(Parent::none());
            parent.register(&child);

            children.push(child);
        }

        let runtime = Runtime::new().unwrap();
        let msg = BastionMessage::start();

        parent.send_children(msg.clone());
        runtime.block_on(async {
            for child in &mut children {
                match poll!(child.next()) {
                    Poll::Ready(Some(BastionMessage::Start)) => (),
                    _ => panic!(),
                }
            }
        });

        parent.unregister(children[0].id());
        parent.send_children(msg.clone());
        runtime.block_on(async {
            assert!(poll!(children[0].next()).is_pending());

            for child in &mut children[1..] {
                match poll!(child.next()) {
                    Poll::Ready(Some(BastionMessage::Start)) => (),
                    _ => panic!(),
                }
            }
        });

        parent.clear_children();
        parent.send_children(msg);
        runtime.block_on(async {
            for child in &mut children[1..] {
                assert!(poll!(child.next()).is_pending());
            }
        });
    }
}
