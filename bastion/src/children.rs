use crate::bastion::REGISTRY;
use crate::broadcast::{BastionMessage, Broadcast, Parent, Sender};
use crate::context::{BastionContext, BastionId, ContextState};
use crate::supervisor::SupervisorRef;
use futures::future::CatchUnwind;
use futures::pending;
use futures::poll;
use futures::prelude::*;
use futures::stream::FuturesUnordered;
use fxhash::FxHashMap;
use qutex::Qutex;
use runtime::task::JoinHandle;
use std::any::Any;
use std::fmt::Debug;
use std::future::Future;
use std::iter::FromIterator;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::task::Poll;

pub trait Shell: objekt::Clone + Send + Sync + Any + 'static {}
impl<T> Shell for T where T: objekt::Clone + Send + Sync + Any + 'static {}

pub trait Message: Shell + Debug {
    fn as_any(&self) -> &dyn Any;
}
impl<T> Message for T
where
    T: Shell + Debug,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub trait Closure: Fn(BastionContext) -> Fut + Shell {}
impl<T> Closure for T where T: Fn(BastionContext) -> Fut + Shell {}

// TODO: Ok(T) & Err(E)
type FutInner = dyn Future<Output = Result<(), ()>> + Send;
type Exec = CatchUnwind<AssertUnwindSafe<Pin<Box<FutInner>>>>;

pub struct Fut(Pin<Box<FutInner>>);

impl<T> From<T> for Fut
where
    T: Future<Output = Result<(), ()>> + Send + 'static,
{
    fn from(fut: T) -> Fut {
        Fut(Box::pin(fut))
    }
}

pub(super) struct Children {
    bcast: Broadcast,
    supervisor: SupervisorRef,
    launched: FxHashMap<BastionId, JoinHandle<Child>>,
    thunk: Box<dyn Closure>,
    redundancy: usize,
    pre_start_msgs: Vec<BastionMessage>,
    started: bool,
}

#[derive(Clone)]
pub struct ChildrenRef {
    id: BastionId,
    sender: Sender,
}

pub(super) struct Child {
    bcast: Broadcast,
    exec: Exec,
    state: Qutex<ContextState>,
    pre_start_msgs: Vec<BastionMessage>,
    started: bool,
}

impl Children {
    pub(super) fn new(
        thunk: Box<dyn Closure>,
        bcast: Broadcast,
        supervisor: SupervisorRef,
        redundancy: usize,
    ) -> Self {
        let launched = FxHashMap::default();
        let pre_start_msgs = Vec::new();
        let started = false;

        let children = Children {
            bcast,
            supervisor,
            launched,
            thunk,
            redundancy,
            pre_start_msgs,
            started,
        };

        REGISTRY.add_children(&children);

        children
    }

    pub(super) async fn reset(&mut self, bcast: Broadcast, supervisor: SupervisorRef) {
        // TODO: stop or kill?
        self.kill().await;

        self.bcast = bcast;
        self.supervisor = supervisor;
    }

    pub fn as_ref(&self) -> ChildrenRef {
        // TODO: clone or ref?
        let id = self.bcast.id().clone();
        let sender = self.bcast.sender().clone();

        ChildrenRef { id, sender }
    }

    pub(super) fn id(&self) -> &BastionId {
        self.bcast.id()
    }

    pub(super) fn sender(&self) -> &Sender {
        self.bcast.sender()
    }

    pub(super) fn bcast(&self) -> &Broadcast {
        &self.bcast
    }

    async fn stop(&mut self) {
        self.bcast.stop_children();

        let launched = self.launched.drain().map(|(_, launched)| launched);
        FuturesUnordered::from_iter(launched)
            .collect::<Vec<_>>()
            .await;
    }

    async fn kill(&mut self) {
        self.bcast.kill_children();

        let launched = self.launched.drain().map(|(_, launched)| launched);
        FuturesUnordered::from_iter(launched)
            .collect::<Vec<_>>()
            .await;
    }

    fn dead(&mut self) {
        REGISTRY.remove_children(&self);

        self.bcast.dead();
    }

    fn faulted(&mut self) {
        REGISTRY.remove_children(&self);

        self.bcast.faulted();
    }

    async fn handle(&mut self, msg: BastionMessage) -> Result<(), ()> {
        match msg {
            BastionMessage::Start => unreachable!(),
            BastionMessage::Stop => {
                self.stop().await;
                self.dead();

                return Err(());
            }
            BastionMessage::PoisonPill => {
                self.kill().await;
                self.dead();

                return Err(());
            }
            // FIXME
            BastionMessage::Deploy(_) => unimplemented!(),
            // FIXME
            BastionMessage::Prune { .. } => unimplemented!(),
            // FIXME
            BastionMessage::SuperviseWith(_) => unimplemented!(),
            BastionMessage::Message(_) => {
                self.bcast.send_children(msg);
            }
            BastionMessage::Dead { id } => {
                // FIXME: Err if false?
                if self.launched.contains_key(&id) {
                    // TODO: stop or kill?
                    self.kill().await;
                    self.dead();

                    return Err(());
                }
            }
            BastionMessage::Faulted { id } => {
                // FIXME: Err if false?
                if self.launched.contains_key(&id) {
                    // TODO: stop or kill?
                    self.kill().await;
                    self.faulted();

                    return Err(());
                }
            }
        }

        Ok(())
    }

    pub(super) async fn run(mut self) -> Self {
        for _ in 0..self.redundancy {
            let parent = Parent::children(self.as_ref());
            let bcast = Broadcast::new(parent);
            let id = bcast.id().clone();

            let children = self.as_ref();
            let supervisor = self.supervisor.clone();

            let state = ContextState::new();
            let state = Qutex::new(state);

            let thunk = objekt::clone_box(&*self.thunk);
            let ctx = BastionContext::new(id, children, supervisor, state.clone());
            let exec = AssertUnwindSafe(thunk(ctx).0).catch_unwind();

            self.bcast.register(&bcast);

            let child = Child::new(exec, bcast, state);
            runtime::spawn(child.run());
        }

        loop {
            match poll!(&mut self.bcast.next()) {
                // TODO: Err if started == true?
                Poll::Ready(Some(BastionMessage::Start)) => {
                    self.started = true;

                    let msgs = self.pre_start_msgs.drain(..).collect::<Vec<_>>();
                    self.pre_start_msgs.shrink_to_fit();

                    for msg in msgs {
                        if self.handle(msg).await.is_err() {
                            return self;
                        }
                    }

                    let msg = BastionMessage::start();
                    self.bcast.send_children(msg);
                }
                Poll::Ready(Some(msg)) if !self.started => {
                    self.pre_start_msgs.push(msg);
                }
                Poll::Ready(Some(msg)) => {
                    if self.handle(msg).await.is_err() {
                        return self;
                    }
                }
                Poll::Ready(None) => {
                    // TODO: stop or kill?
                    self.kill().await;
                    self.faulted();

                    return self;
                }
                Poll::Pending => pending!(),
            }
        }
    }
}

impl ChildrenRef {
    pub(super) fn send(&self, msg: BastionMessage) {
        // FIXME: Err(Error)
        self.sender.unbounded_send(msg).ok();
    }
}

impl Child {
    fn new(exec: Exec, bcast: Broadcast, state: Qutex<ContextState>) -> Self {
        let pre_start_msgs = Vec::new();
        let started = false;

        let child = Child {
            bcast,
            exec,
            state,
            pre_start_msgs,
            started,
        };

        REGISTRY.add_child(&child);

        child
    }

    pub(super) fn id(&self) -> &BastionId {
        self.bcast.id()
    }

    pub(super) fn sender(&self) -> &Sender {
        self.bcast.sender()
    }

    fn dead(&mut self) {
        REGISTRY.remove_child(&self);

        self.bcast.dead();
    }

    fn faulted(&mut self) {
        REGISTRY.remove_child(&self);

        self.bcast.faulted();
    }

    async fn handle(&mut self, msg: BastionMessage) -> Result<(), ()> {
        match msg {
            BastionMessage::Start => unreachable!(),
            BastionMessage::Stop | BastionMessage::PoisonPill => {
                self.dead();

                return Err(());
            }
            // FIXME
            BastionMessage::Deploy(_) => unimplemented!(),
            // FIXME
            BastionMessage::Prune { .. } => unimplemented!(),
            // FIXME
            BastionMessage::SuperviseWith(_) => unimplemented!(),
            BastionMessage::Message(msg) => {
                let mut state = self.state.clone().lock_async().await.map_err(|_| ())?;
                state.push_msg(msg);
            }
            // FIXME
            BastionMessage::Dead { .. } => unimplemented!(),
            // FIXME
            BastionMessage::Faulted { .. } => unimplemented!(),
        }

        Ok(())
    }

    async fn run(mut self) {
        loop {
            match poll!(&mut self.bcast.next()) {
                // TODO: Err if started == true?
                Poll::Ready(Some(BastionMessage::Start)) => {
                    self.started = true;

                    let msgs = self.pre_start_msgs.drain(..).collect::<Vec<_>>();
                    self.pre_start_msgs.shrink_to_fit();

                    for msg in msgs {
                        if self.handle(msg).await.is_err() {
                            return;
                        }
                    }

                    continue;
                }
                Poll::Ready(Some(msg)) if !self.started => {
                    self.pre_start_msgs.push(msg);

                    continue;
                }
                Poll::Ready(Some(msg)) => {
                    if self.handle(msg).await.is_err() {
                        return;
                    }

                    continue;
                }
                Poll::Ready(None) => {
                    self.faulted();

                    return;
                }
                Poll::Pending => (),
            }

            if !self.started {
                pending!();

                continue;
            }

            if let Poll::Ready(res) = poll!(&mut self.exec) {
                match res {
                    Ok(Ok(())) => return self.dead(),
                    Ok(Err(())) | Err(_) => return self.faulted(),
                }
            }

            pending!();
        }
    }
}
