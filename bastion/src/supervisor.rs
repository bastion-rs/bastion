use crate::broadcast::{BastionMessage, Broadcast};
use crate::children::{Children, Closure, Message};
use futures::pending;
use futures::poll;
use futures::prelude::*;
use fxhash::FxHashMap;
use fxhash::FxHashSet;
use runtime::task::JoinHandle;
use std::collections::BTreeMap;
use std::ops::RangeFrom;
use std::task::Poll;
use uuid::Uuid;


pub struct Supervisor {
    bcast: Broadcast,
    children: Vec<Children>,
    order: BTreeMap<usize, Uuid>, // FIXME
    launched: FxHashMap<Uuid, (usize, JoinHandle<Children>)>, // FIXME
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
        let order = BTreeMap::new();
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

    fn launch_children(&mut self) {
        let start = if let Some(order) = self.order.keys().next_back() {
            *order + 1
        } else {
            0
        };

        for (order, children) in self.children.drain(..).enumerate() {
            let id = children.id().clone();
            let order = start + order;

            self.order.insert(order, id.clone());
            self.launched.insert(id, (order, children.launch()));
        }
    }

    async fn kill_children(&mut self, range: RangeFrom<usize>) {
        if range.start == 0 {
            self.bcast.send_children(BastionMessage::poison_pill());
            self.bcast.clear_children();
        } else {
            for (_, id) in self.order.range(range.clone()) {
                self.bcast.send_child(id, BastionMessage::poison_pill());
                self.bcast.remove_child(id);
            }
        }

        let mut removed = vec![];
        let mut children = Vec::new();
        for (order, id) in self.order.range(range) {
            // FIXME: Err if None?
            if let Some((_, launched)) = self.launched.remove(id) {
                // FIXME: join?
                children.push(launched.await);
            }

            removed.push(*order);
        }

        for order in removed {
            self.order.remove(&order);
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

                self.kill_children(order..);
                self.launch_children();
            }
        }

        Ok(())
    }

    async fn run(mut self) -> Self {
        loop {
            match poll!(&mut self.bcast.next()) {
                Poll::Ready(Some(msg)) => {
                    match msg {
                        BastionMessage::PoisonPill => {
                            let id = self.bcast.id().clone();

                            self.bcast.send_children(BastionMessage::poison_pill());
                            self.bcast.clear_children();

                            self.bcast.send_parent(BastionMessage::dead(id));

                            return self;
                        }
                        BastionMessage::Dead { id } => {
                            self.launched.remove(&id);
                            self.bcast.remove_child(&id);

                            self.dead.insert(id);
                        }
                        BastionMessage::Faulted { id } => {
                            if self.recover(id).await.is_err() {
                                self.bcast.send_children(BastionMessage::poison_pill());
                                self.bcast.clear_children();

                                let id = self.bcast.id().clone();
                                self.bcast.send_parent(BastionMessage::faulted(id));

                                return self;
                            }
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

    pub fn launch(mut self) -> JoinHandle<Self> {
        //self.launch_children();

        runtime::spawn(self.run())
    }
}

impl Default for SupervisionStrategy {
    fn default() -> Self {
        SupervisionStrategy::OneForOne
    }
}
