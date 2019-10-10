use crate::bastion::SYSTEM;
use crate::broadcast::{BastionMessage, Broadcast, Sender};
use crate::children::{Children, Closure, Message};
use futures::{pending, poll};
use futures::prelude::*;
use fxhash::{FxHashMap, FxHashSet};
use runtime::task::JoinHandle;
use std::ops::RangeFrom;
use std::task::Poll;
use uuid::Uuid;

pub struct Supervisor {
    bcast: Broadcast,
    children: Vec<Children>,
    // FIXME: contains dead Children
    order: Vec<Uuid>,
    launched: FxHashMap<Uuid, (usize, JoinHandle<Children>)>,
    dead: FxHashSet<Uuid>,
    strategy: SupervisionStrategy,
}

pub enum SupervisionStrategy {
    OneForOne,
    OneForAll,
    RestForOne,
}

impl Supervisor {
    pub(super) fn new() -> Self {
        let id = Uuid::new_v4();
        let bcast = Broadcast::new(id);

        let children = Vec::new();
        let order = Vec::new();
        let launched = FxHashMap::default();
        let dead = FxHashSet::default();
        let strategy = SupervisionStrategy::default();

        Supervisor {
            bcast,
            children,
            order,
            launched,
            dead,
            strategy,
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.bcast.id()
    }

    pub(super) fn sender(&self) -> &Sender {
        self.bcast.sender()
    }

    pub fn strategy(mut self, strategy: SupervisionStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn children<F, M>(mut self, thunk: F, msg: M, redundancy: usize) -> Self
        where
            F: Closure,
            M: Message,
    {
        let id = Uuid::new_v4();
        let bcast = self.bcast.new_child(id);

        let thunk = Box::new(thunk);
        let msg = Box::new(msg);

        let children = Children::new(
            thunk,
            msg,
            bcast,
            redundancy,
        );

        self.children.push(children);

        self
    }

    pub(super) fn launch_children(&mut self) {
        for children in self.children.drain(..) {
            let id = children.id().clone();

            self.launched.insert(id.clone(), (self.order.len(), children.launch()));
            self.order.push(id);
        }
    }

    async fn kill_children(&mut self, range: RangeFrom<usize>) {
        if range.start == 0 {
	        self.bcast.poison_pill_children();
        } else {
            // FIXME: panics
            for id in self.order.get(range.clone()).unwrap() {
	            self.bcast.poison_pill_child(id);
            }
        }

        let mut children = Vec::new();
        for id in self.order.drain(range) {
            // FIXME: Err if None?
            if let Some((_, launched)) = self.launched.remove(&id) {
                // FIXME: join?
                children.push(launched.await);
            }
        }

        // FIXME: might remove children
        self.children = children;
    }

    async fn recover(&mut self, id: Uuid) -> Result<(), ()> {
        match self.strategy {
            SupervisionStrategy::OneForOne => {
                let (order, launched) = self.launched.remove(&id).ok_or(())?;
                let children = launched.await;

                self.launched.insert(id, (order, children.launch()));
            }
            SupervisionStrategy::OneForAll => {
                self.kill_children(0..).await;
                self.launch_children();
            }
            SupervisionStrategy::RestForOne => {
                let (order, launched) = self.launched.remove(&id).ok_or(())?;
                let children = launched.await;

                self.children.push(children);

                self.kill_children(order..).await;
                self.launch_children();
            }
        }

        Ok(())
    }

    pub(super) async fn run(mut self) -> Self {
        loop {
            match poll!(&mut self.bcast.next()) {
                Poll::Ready(Some(msg)) => {
                    match msg {
                        BastionMessage::PoisonPill => {
                            self.bcast.dead();

                            return self;
                        }
                        BastionMessage::Dead { id } => {
                            self.launched.remove(&id);
                            self.bcast.remove_child(&id);

                            self.dead.insert(id);
                        }
                        BastionMessage::Faulted { id } => {
                            if self.recover(id).await.is_err() {
                                self.bcast.faulted();

                                return self;
                            }
                        }
                        BastionMessage::Message(_) => {
                            // TODO: send to parent too?

                            self.bcast.send_children(msg);
                        }
                    }
                }
                Poll::Ready(None) => {
                    self.bcast.faulted();

                    return self;
                }
                Poll::Pending => pending!(),
            }
        }
    }

    pub fn launch(self) {
        // FIXME: handle errors
        SYSTEM.unbounded_send(self).ok();
    }
}

impl Default for SupervisionStrategy {
    fn default() -> Self {
        SupervisionStrategy::OneForOne
    }
}
