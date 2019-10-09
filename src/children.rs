use crate::broadcast::{BastionMessage, Broadcast};
use crate::context::BastionContext;
use futures::future::CatchUnwind;
use futures::pending;
use futures::poll;
use futures::prelude::*;
use runtime::task::JoinHandle;
use std::any::Any;
use std::fmt::Debug;
use std::future::Future;
use std::panic::UnwindSafe;
use std::pin::Pin;
use std::task::Poll;
use uuid::Uuid;

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

struct Child {
    exec: CatchUnwind<Pin<Box<dyn Fut>>>,
    bcast: Broadcast,
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

    pub(super) fn id(&self) -> &Uuid {
        self.bcast.id()
    }

    async fn run(mut self) -> Self {
        loop {
            match poll!(&mut self.bcast.next()) {
                Poll::Ready(Some(msg)) => {
                    let id = self.bcast.id().clone();

                    match msg {
                        BastionMessage::PoisonPill | BastionMessage::Dead { .. } | BastionMessage::Faulted { .. } => {
                            self.bcast.send_children(BastionMessage::poison_pill());
                            self.bcast.clear_children();

                            if msg.is_faulted() {
                                self.bcast.send_parent(BastionMessage::faulted(id));
                            } else {
                                self.bcast.send_parent(BastionMessage::dead(id));
                            }

                            return self;
                        }
                        // FIXME
                        BastionMessage::Message(_) => unimplemented!(),
                    }
                }
                // FIXME: "return self" or "send_parent(Faulted)"?
                Poll::Ready(None) => unimplemented!(),
                Poll::Pending => pending!(),
            }
        }
    }

    pub(super) fn launch(mut self) ->  JoinHandle<Self> {
        for _ in 0..self.redundancy {
            let id = Uuid::new_v4();
            let bcast = self.bcast.new_child(id);

            let thunk = objekt::clone_box(&*self.thunk);
            let ctx = BastionContext { };
            let msg = objekt::clone_box(&*self.msg);
            let exec = thunk(ctx, msg)
                .catch_unwind();

            let child = Child { exec, bcast };
            runtime::spawn(child.run());
        }

        runtime::spawn(self.run())
    }
}

impl Child {
    async fn run(mut self) {
        loop {
            if let Poll::Ready(res) = poll!(&mut self.exec) {
                let id = self.bcast.id().clone();

                match res {
                    Ok(Ok(())) => self.bcast.send_parent(BastionMessage::dead(id)),
                    Ok(Err(())) | Err(_) => self.bcast.send_parent(BastionMessage::faulted(id)),
                }

                return;
            }

            match poll!(&mut self.bcast.next()) {
                Poll::Ready(Some(msg)) => {
                    let id = self.bcast.id().clone();

                    match msg {
                        BastionMessage::PoisonPill => {
                            self.bcast.send_parent(BastionMessage::dead(id));

                            return;
                        }
                        BastionMessage::Dead { .. } | BastionMessage::Faulted { .. } => {
                            // FIXME: shouldn't happen; send_parent(Faulted)?
                            unimplemented!()
                        }
                        // FIXME
                        BastionMessage::Message(_) => unimplemented!(),
                    }
                }
                // FIXME: "return" or "send_parent(Faulted)"?
                Poll::Ready(None) => unimplemented!(),
                Poll::Pending => pending!(),
            }
        }
    }
}
