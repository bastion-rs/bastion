use crate::bastion::REGISTRY;
use crate::broadcast::{BastionMessage, Broadcast, Deployment, Parent, Sender};
use crate::children::{Children, Closure};
use crate::context::BastionId;
use crate::proc::Proc;
use futures::prelude::*;
use futures::stream::FuturesUnordered;
use futures::{pending, poll};
use fxhash::FxHashMap;
use std::ops::RangeFrom;
use std::task::Poll;

#[derive(Debug)]
pub struct Supervisor {
    bcast: Broadcast,
    order: Vec<BastionId>,
    launched: FxHashMap<BastionId, (usize, Proc<Supervised>)>,
    dead: FxHashMap<BastionId, Supervised>,
    strategy: SupervisionStrategy,
    pre_start_msgs: Vec<BastionMessage>,
    started: bool,
}

#[derive(Debug, Clone)]
pub struct SupervisorRef {
    id: BastionId,
    sender: Sender,
}

#[derive(Debug, Clone)]
pub enum SupervisionStrategy {
    OneForOne,
    OneForAll,
    RestForOne,
}

#[derive(Debug)]
enum Supervised {
    Supervisor(Supervisor),
    Children(Children),
}

impl Supervisor {
    pub(super) fn new(bcast: Broadcast) -> Self {
        let order = Vec::new();
        let launched = FxHashMap::default();
        let dead = FxHashMap::default();
        let strategy = SupervisionStrategy::default();
        let pre_start_msgs = Vec::new();
        let started = false;

        let supervisor = Supervisor {
            bcast,
            order,
            launched,
            dead,
            strategy,
            pre_start_msgs,
            started,
        };

        REGISTRY.add_supervisor(&supervisor);

        supervisor
    }

    pub(super) async fn reset(&mut self, bcast: Broadcast) {
        // TODO: stop or kill?
        let supervised = self.kill(0..).await;

        REGISTRY.remove_supervisor(&self);

        self.bcast = bcast;
        self.pre_start_msgs.clear();
        self.pre_start_msgs.shrink_to_fit();

        REGISTRY.add_supervisor(&self);

        let mut reset = FuturesUnordered::new();
        for supervised in supervised {
            let parent = Parent::supervisor(self.as_ref());
            let bcast = Broadcast::new(parent);
            let supervisor = self.as_ref();

            reset.push(supervised.reset(bcast, supervisor))
        }

        while let Some(supervised) = reset.next().await {
            let id = supervised.id().clone();

            let launched = supervised.launch();
            self.launched
                .insert(id.clone(), (self.order.len(), launched));
            self.order.push(id);
        }

        if self.started {
            let msg = BastionMessage::start();
            self.bcast.send_children(msg);
        }
    }

    pub fn as_ref(&self) -> SupervisorRef {
        // TODO: clone or ref?
        let id = self.bcast.id().clone();
        let sender = self.bcast.sender().clone();

        SupervisorRef { id, sender }
    }

    pub fn id(&self) -> &BastionId {
        &self.bcast.id()
    }

    pub(super) fn sender(&self) -> &Sender {
        self.bcast.sender()
    }

    pub(super) fn bcast(&self) -> &Broadcast {
        &self.bcast
    }

    pub fn supervisor<S>(mut self, init: S) -> Self
    where
        S: FnOnce(Supervisor) -> Supervisor,
    {
        let parent = Parent::supervisor(self.as_ref());
        let bcast = Broadcast::new(parent);

        let supervisor = Supervisor::new(bcast);
        let supervisor = init(supervisor);
        self.bcast.register(&supervisor.bcast);

        let msg = BastionMessage::deploy_supervisor(supervisor);
        self.bcast.send_self(msg);

        self
    }

    pub fn children<F>(self, thunk: F, redundancy: usize) -> Self
    where
        F: Closure,
    {
        let parent = Parent::supervisor(self.as_ref());
        let bcast = Broadcast::new(parent);
        let thunk = Box::new(thunk);

        let children = Children::new(thunk, bcast, self.as_ref(), redundancy);
        let msg = BastionMessage::deploy_children(children);
        self.bcast.send_self(msg);

        self
    }

    pub fn strategy(mut self, strategy: SupervisionStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    async fn stop(&mut self, range: RangeFrom<usize>) -> Vec<Supervised> {
        if range.start == 0 {
            self.bcast.stop_children();
        } else {
            // FIXME: panics
            for id in self.order.get(range.clone()).unwrap() {
                self.bcast.stop_child(id);
            }
        }

        self.collect(range).await
    }

    async fn kill(&mut self, range: RangeFrom<usize>) -> Vec<Supervised> {
        if range.start == 0 {
            self.bcast.kill_children();
        } else {
            // FIXME: panics
            for id in self.order.get(range.clone()).unwrap() {
                self.bcast.kill_child(id);
            }
        }

        self.collect(range).await
    }

    fn dead(&mut self) {
        REGISTRY.remove_supervisor(&self);

        self.bcast.dead();
    }

    fn faulted(&mut self) {
        REGISTRY.remove_supervisor(&self);

        self.bcast.faulted();
    }

    async fn collect(&mut self, range: RangeFrom<usize>) -> Vec<Supervised> {
        let mut dead = Vec::new();
        let mut supervised = FuturesUnordered::new();
        for id in self.order.drain(range) {
            // TODO: Err if None?
            if let Some((_, launched)) = self.launched.remove(&id) {
                // TODO: add a "dead" list and poll from it instead of awaiting
                supervised.push(launched);
            }

            if let Some(supervised) = self.dead.remove(&id) {
                dead.push(supervised);
            }
        }

        supervised.collect().await
    }

    async fn recover(&mut self, id: BastionId) -> Result<(), ()> {
        match self.strategy {
            SupervisionStrategy::OneForOne => {
                let (order, launched) = self.launched.remove(&id).ok_or(())?;
                // TODO: add a "waiting" list and poll from it instead of awaiting
                let supervised = launched.await;

                self.bcast.unregister(supervised.id());

                let parent = Parent::supervisor(self.as_ref());
                let bcast = Broadcast::new(parent);
                let id = bcast.id().clone();
                let supervised = supervised.reset(bcast, self.as_ref()).await;

                self.bcast.register(supervised.bcast());
                if self.started {
                    let msg = BastionMessage::start();
                    self.bcast.send_child(&id, msg);
                }

                let launched = supervised.launch();
                self.launched.insert(id, (order, launched));
            }
            SupervisionStrategy::OneForAll => {
                // TODO: stop or kill?
                for supervised in self.kill(0..).await {
                    self.bcast.unregister(supervised.id());

                    let parent = Parent::supervisor(self.as_ref());
                    let bcast = Broadcast::new(parent);
                    let id = bcast.id().clone();
                    let supervised = supervised.reset(bcast, self.as_ref()).await;

                    self.bcast.register(supervised.bcast());

                    let launched = supervised.launch();
                    self.launched
                        .insert(id.clone(), (self.order.len(), launched));
                    self.order.push(id);
                }

                if self.started {
                    let msg = BastionMessage::start();
                    self.bcast.send_children(msg);
                }
            }
            SupervisionStrategy::RestForOne => {
                let (order, _) = self.launched.get(&id).ok_or(())?;
                let order = *order;

                // TODO: stop or kill?
                for supervised in self.kill(order..).await {
                    self.bcast.unregister(supervised.id());

                    let parent = Parent::supervisor(self.as_ref());
                    let bcast = Broadcast::new(parent);
                    let id = bcast.id().clone();
                    let supervised = supervised.reset(bcast, self.as_ref()).await;

                    self.bcast.register(supervised.bcast());
                    if self.started {
                        let msg = BastionMessage::start();
                        self.bcast.send_child(&id, msg);
                    }

                    let launched = supervised.launch();
                    self.launched
                        .insert(id.clone(), (self.order.len(), launched));
                    self.order.push(id);
                }
            }
        }

        Ok(())
    }

    async fn handle(&mut self, msg: BastionMessage) -> Result<(), ()> {
        match msg {
            BastionMessage::Start => unreachable!(),
            BastionMessage::Stop => {
                self.stop(0..).await;
                self.dead();

                return Err(());
            }
            BastionMessage::PoisonPill => {
                self.kill(0..).await;
                self.dead();

                return Err(());
            }
            BastionMessage::Deploy(deployment) => match deployment {
                Deployment::Supervisor(supervisor) => {
                    self.bcast.register(&supervisor.bcast);

                    let id = supervisor.id().clone();
                    let supervised = Supervised::supervisor(supervisor);

                    let launched = supervised.launch();
                    self.launched
                        .insert(id.clone(), (self.order.len(), launched));
                    self.order.push(id);
                }
                Deployment::Children(children) => {
                    self.bcast.register(children.bcast());

                    let id = children.id().clone();
                    let supervised = Supervised::children(children);

                    let launched = supervised.launch();
                    self.launched
                        .insert(id.clone(), (self.order.len(), launched));
                    self.order.push(id);
                }
            },
            // FIXME
            BastionMessage::Prune { .. } => unimplemented!(),
            BastionMessage::SuperviseWith(strategy) => {
                self.strategy = strategy;
            }
            BastionMessage::Message(_) => {
                self.bcast.send_children(msg);
            }
            BastionMessage::Dead { id } => {
                // FIXME: Err if None?
                if let Some((_, launched)) = self.launched.remove(&id) {
                    // TODO: add a "waiting" list an poll from it instead of awaiting
                    let supervised = launched.await;

                    self.bcast.unregister(&id);
                    self.dead.insert(id, supervised);
                }
            }
            BastionMessage::Faulted { id } => {
                if self.recover(id).await.is_err() {
                    self.kill(0..).await;
                    self.faulted();

                    return Err(());
                }
            }
        }

        Ok(())
    }

    pub(super) async fn run(mut self) -> Self {
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
                    self.kill(0..).await;
                    self.faulted();

                    return self;
                }
                Poll::Pending => pending!(),
            }
        }
    }
}

impl SupervisorRef {
    pub(super) fn new(id: BastionId, sender: Sender) -> Self {
        SupervisorRef { id, sender }
    }

    // TODO: Err(Error)?
    pub fn supervisor<S>(&mut self, init: S)
    where
        S: FnOnce(Supervisor) -> Supervisor,
    {
        let parent = Parent::supervisor(self.clone());
        let bcast = Broadcast::new(parent);

        let supervisor = Supervisor::new(bcast);
        let supervisor = init(supervisor);
        let msg = BastionMessage::deploy_supervisor(supervisor);
        self.send(msg);
    }

    pub fn children<F>(&mut self, thunk: F, redundancy: usize)
    where
        F: Closure,
    {
        let parent = Parent::supervisor(self.clone());
        let bcast = Broadcast::new(parent);
        let thunk = Box::new(thunk);

        let children = Children::new(thunk, bcast, self.clone(), redundancy);
        let msg = BastionMessage::deploy_children(children);
        self.send(msg);
    }

    pub fn strategy(&mut self, strategy: SupervisionStrategy) {
        let msg = BastionMessage::supervise_with(strategy);
        self.send(msg);
    }

    pub(super) fn send(&self, msg: BastionMessage) {
        // FIXME: Err(Error)
        self.sender.unbounded_send(msg).ok();
    }
}

impl Supervised {
    fn supervisor(supervisor: Supervisor) -> Self {
        Supervised::Supervisor(supervisor)
    }

    fn children(children: Children) -> Self {
        Supervised::Children(children)
    }

    fn id(&self) -> &BastionId {
        match self {
            Supervised::Supervisor(supervisor) => supervisor.id(),
            Supervised::Children(children) => children.id(),
        }
    }

    fn bcast(&self) -> &Broadcast {
        match self {
            Supervised::Supervisor(supervisor) => supervisor.bcast(),
            Supervised::Children(children) => children.bcast(),
        }
    }

    fn reset(self, bcast: Broadcast, supervisor: SupervisorRef) -> Proc<Self> {
        match self {
            Supervised::Supervisor(mut supervisor) => Proc::spawn(async {
                supervisor.reset(bcast).await;
                Supervised::Supervisor(supervisor)
            }),
            Supervised::Children(mut children) => Proc::spawn(async {
                children.reset(bcast, supervisor).await;
                Supervised::Children(children)
            }),
        }
    }

    fn launch(self) -> Proc<Self> {
        match self {
            Supervised::Supervisor(supervisor) => Proc::spawn(async {
                let supervisor = supervisor.run().await;
                Supervised::Supervisor(supervisor)
            }),
            Supervised::Children(children) => Proc::spawn(async {
                let children = children.run().await;
                Supervised::Children(children)
            }),
        }
    }
}

impl Default for SupervisionStrategy {
    fn default() -> Self {
        SupervisionStrategy::OneForOne
    }
}
