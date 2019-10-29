use crate::children::{Children, ChildrenRef, Message, Msg};
use crate::context::BastionId;
use crate::supervisor::{SupervisionStrategy, Supervisor, SupervisorRef};
use crate::system::SYSTEM;
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::prelude::*;
use fxhash::FxHashMap;
use std::any::Any;
use std::pin::Pin;
use std::task::{Context, Poll};

pub(crate) type Sender = UnboundedSender<BastionMessage>;
pub(crate) type Receiver = UnboundedReceiver<BastionMessage>;

#[derive(Debug)]
pub(crate) struct Broadcast {
    id: BastionId,
    sender: Sender,
    recver: Receiver,
    parent: Parent,
    children: FxHashMap<BastionId, Sender>,
}

#[derive(Debug)]
pub(crate) enum Parent {
    None,
    System,
    Supervisor(SupervisorRef),
    Children(ChildrenRef),
}

#[derive(Debug)]
pub(crate) enum BastionMessage {
    Start,
    Stop,
    Kill,
    Deploy(Deployment),
    Prune { id: BastionId },
    SuperviseWith(SupervisionStrategy),
    Tell(Msg),
    Stopped { id: BastionId },
    Faulted { id: BastionId },
}

#[derive(Debug)]
pub(crate) enum Deployment {
    Supervisor(Supervisor),
    Children(Children),
}

impl Broadcast {
    pub(crate) fn new(parent: Parent) -> Self {
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

    pub(crate) fn with_id(parent: Parent, id: BastionId) -> Self {
        let mut bcast = Broadcast::new(parent);
        bcast.id = id;

        bcast
    }

    pub(crate) fn id(&self) -> &BastionId {
        &self.id
    }

    pub(crate) fn sender(&self) -> &Sender {
        &self.sender
    }

    pub(crate) fn register(&mut self, child: &Self) {
        self.children.insert(child.id.clone(), child.sender.clone());
    }

    pub(crate) fn unregister(&mut self, id: &BastionId) {
        self.children.remove(id);
    }

    pub(crate) fn clear_children(&mut self) {
        self.children.clear();
    }

    pub(crate) fn stop_child(&mut self, id: &BastionId) {
        let msg = BastionMessage::stop();
        self.send_child(id, msg);

        self.unregister(id);
    }

    pub(crate) fn stop_children(&mut self) {
        let msg = BastionMessage::stop();
        self.send_children(msg);

        self.clear_children();
    }

    pub(crate) fn kill_child(&mut self, id: &BastionId) {
        let msg = BastionMessage::kill();
        self.send_child(id, msg);

        self.unregister(id);
    }

    pub(crate) fn kill_children(&mut self) {
        let msg = BastionMessage::kill();
        self.send_children(msg);

        self.clear_children();
    }

    pub(crate) fn stopped(&mut self) {
        self.stop_children();

        let msg = BastionMessage::stopped(self.id.clone());
        // FIXME: Err(msg)
        self.send_parent(msg).ok();
    }

    pub(crate) fn faulted(&mut self) {
        self.kill_children();

        let msg = BastionMessage::faulted(self.id.clone());
        // FIXME: Err(msg)
        self.send_parent(msg).ok();
    }

    pub(crate) fn send_parent(&self, msg: BastionMessage) -> Result<(), BastionMessage> {
        self.parent.send(msg)
    }

    pub(crate) fn send_child(&self, id: &BastionId, msg: BastionMessage) {
        // FIXME: Err if None?
        if let Some(child) = self.children.get(id) {
            // FIXME: handle errors
            child.unbounded_send(msg).ok();
        }
    }

    pub(crate) fn send_children(&self, msg: BastionMessage) {
        for (_, child) in &self.children {
            // FIXME: Err(Error) if None
            if let Some(msg) = msg.try_clone() {
                // FIXME: handle errors
                child.unbounded_send(msg).ok();
            }
        }
    }

    pub(crate) fn send_self(&self, msg: BastionMessage) {
        // FIXME: handle errors
        self.sender.unbounded_send(msg).ok();
    }
}

impl Parent {
    pub(crate) fn none() -> Self {
        Parent::None
    }

    pub(crate) fn system() -> Self {
        Parent::System
    }

    pub(crate) fn supervisor(supervisor: SupervisorRef) -> Self {
        Parent::Supervisor(supervisor)
    }

    pub(crate) fn children(children: ChildrenRef) -> Self {
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
    pub(crate) fn start() -> Self {
        BastionMessage::Start
    }

    pub(crate) fn stop() -> Self {
        BastionMessage::Stop
    }

    pub(crate) fn kill() -> Self {
        BastionMessage::Kill
    }

    pub(crate) fn deploy_supervisor(supervisor: Supervisor) -> Self {
        let deployment = Deployment::Supervisor(supervisor);

        BastionMessage::Deploy(deployment)
    }

    pub(crate) fn deploy_children(children: Children) -> Self {
        let deployment = Deployment::Children(children);

        BastionMessage::Deploy(deployment)
    }

    pub(crate) fn prune(id: BastionId) -> Self {
        BastionMessage::Prune { id }
    }

    pub(crate) fn supervise_with(strategy: SupervisionStrategy) -> Self {
        BastionMessage::SuperviseWith(strategy)
    }

    pub(crate) fn broadcast<M: Message>(msg: M) -> Self {
        let msg = Msg::shared(msg);
        BastionMessage::Tell(msg)
    }

    pub(crate) fn tell<M: Message>(msg: M) -> Self {
        let msg = Msg::owned(msg);
        BastionMessage::Tell(msg)
    }

    pub(crate) fn stopped(id: BastionId) -> Self {
        BastionMessage::Stopped { id }
    }

    pub(crate) fn faulted(id: BastionId) -> Self {
        BastionMessage::Faulted { id }
    }

    pub(crate) fn try_clone(&self) -> Option<Self> {
        let clone = match self {
            BastionMessage::Start => BastionMessage::start(),
            BastionMessage::Stop => BastionMessage::stop(),
            BastionMessage::Kill => BastionMessage::kill(),
            // FIXME
            BastionMessage::Deploy(_) => unimplemented!(),
            BastionMessage::Prune { id } => BastionMessage::prune(id.clone()),
            BastionMessage::SuperviseWith(strategy) => {
                BastionMessage::supervise_with(strategy.clone())
            }
            BastionMessage::Tell(msg) => BastionMessage::Tell(msg.try_clone()?),
            BastionMessage::Stopped { id } => BastionMessage::stopped(id.clone()),
            BastionMessage::Faulted { id } => BastionMessage::faulted(id.clone()),
        };

        Some(clone)
    }

    pub(crate) fn into_msg<M>(self) -> Option<M>
    where
        M: Any + Send + Sync + 'static,
    {
        if let BastionMessage::Tell(msg) = self {
            msg.try_unwrap().ok()
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

        parent.send_children(msg.try_clone().unwrap());
        runtime.block_on(async {
            for child in &mut children {
                match poll!(child.next()) {
                    Poll::Ready(Some(BastionMessage::Start)) => (),
                    _ => panic!(),
                }
            }
        });

        parent.unregister(children[0].id());
        parent.send_children(msg.try_clone().unwrap());
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
