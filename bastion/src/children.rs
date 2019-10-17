use crate::bastion::REGISTRY;
use crate::broadcast::{BastionMessage, Broadcast, Sender};
use crate::context::{BastionContext, BastionId, ContextState};
use futures::future::CatchUnwind;
use futures::pending;
use futures::poll;
use futures::prelude::*;
use qutex::Qutex;
use runtime::task::JoinHandle;
use std::any::Any;
use std::fmt::Debug;
use std::future::Future;
use std::panic::UnwindSafe;
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

pub trait Closure: Fn(BastionContext, Box<dyn Message>) -> Pin<Box<dyn Fut>> + Shell {}
impl<T> Closure for T where T: Fn(BastionContext, Box<dyn Message>) -> Pin<Box<dyn Fut>> + Shell {}

// TODO: Ok(T) & Err(E)
pub trait Fut: Future<Output = Result<(), ()>> + Send + UnwindSafe {}
impl<T> Fut for T where T: Future<Output = Result<(), ()>> + Send + UnwindSafe {}

pub(super) struct Children {
    thunk: Box<dyn Closure>,
    msg: Box<dyn Message>,
    bcast: Broadcast,
    redundancy: usize,
}

pub(super) struct Child {
    exec: CatchUnwind<Pin<Box<dyn Fut>>>,
    bcast: Broadcast,
    state: Qutex<ContextState>,
}

impl Children {
    pub(super) fn new(
        thunk: Box<dyn Closure>,
        msg: Box<dyn Message>,
        bcast: Broadcast,
        redundancy: usize,
    ) -> Self {
        Children {
            thunk,
            msg,
            bcast,
            redundancy,
        }
    }

    pub(super) fn reset(&mut self, bcast: Broadcast) {
        self.bcast.poison_pill_children();

        self.bcast = bcast;
    }

    pub(super) fn id(&self) -> &BastionId {
        self.bcast.id()
    }

    pub(super) fn sender(&self) -> &Sender {
        self.bcast.sender()
    }

    async fn run(mut self) -> Self {
        REGISTRY.add_children(&self);

        loop {
            match poll!(&mut self.bcast.next()) {
                Poll::Ready(Some(msg)) => {
                    match msg {
                        BastionMessage::PoisonPill | BastionMessage::Dead { .. } | BastionMessage::Faulted { .. } => {
	                        REGISTRY.remove_children(&self);

                            if msg.is_faulted() {
	                            self.bcast.faulted();
                            } else {
                                self.bcast.dead();
                            }

                            return self;
                        }
                        // FIXME
                        BastionMessage::Message(_) => unimplemented!(),
                    }
                }
                Poll::Ready(None) => {
                    REGISTRY.remove_children(&self);

                    self.bcast.faulted();

                    return self;
                }
                Poll::Pending => pending!(),
            }
        }
    }

    pub(super) fn launch(mut self) ->  JoinHandle<Self> {
        for _ in 0..self.redundancy {
            let bcast = self.bcast.new_child();
            let id = bcast.id().clone();

            let state = ContextState::new();
            let state = Qutex::new(state);

            let thunk = objekt::clone_box(&*self.thunk);
            let msg = objekt::clone_box(&*self.msg);

            let parent = self.bcast.sender().clone();
            let ctx = BastionContext::new(id, parent, state.clone());

            let exec = thunk(ctx, msg)
                .catch_unwind();

            let child = Child { exec, bcast, state };
            runtime::spawn(child.run());
        }

        runtime::spawn(self.run())
    }
}

impl Child {
    pub(super) fn id(&self) -> &BastionId {
        self.bcast.id()
    }

    pub(super) fn sender(&self) -> &Sender {
        self.bcast.sender()
    }

    fn dead(mut self) {
        REGISTRY.remove_child(&self);

        self.bcast.dead();
    }

    fn faulted(mut self) {
        REGISTRY.remove_child(&self);

        self.bcast.faulted();
    }

    async fn run(mut self) {
        REGISTRY.add_child(&self);

        loop {
            match poll!(&mut self.bcast.next()) {
                Poll::Ready(Some(msg)) => {
                    // FIXME: Err(Error)
                    let mut state = self.state.clone().lock_async().await.unwrap();

                    match msg {
                        BastionMessage::PoisonPill => return self.dead(),
                        // FIXME
	                    BastionMessage::Dead { .. } => unimplemented!(),
                        // FIXME
                        BastionMessage::Faulted { .. } => unimplemented!(),
                        BastionMessage::Message(msg) => {
                            state.push_msg(msg);

                            continue;
                        },
                    }
                }
                Poll::Ready(None) => return self.faulted(),
                Poll::Pending => (),
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
