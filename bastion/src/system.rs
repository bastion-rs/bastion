use crate::broadcast::{BastionMessage, Broadcast, Deployment, Parent, Sender};
use crate::context::{BastionId, NIL_ID};
use crate::proc::Proc;
use crate::supervisor::Supervisor;
use futures::prelude::*;
use futures::stream::FuturesUnordered;
use futures::{pending, poll};
use fxhash::{FxHashMap, FxHashSet};
use lazy_static::lazy_static;
use std::task::Poll;
use tokio::runtime::{Builder, Runtime};

lazy_static! {
    pub(super) static ref RUNTIME: Runtime = Builder::new().panic_handler(|_| ()).build().unwrap();
}

pub(super) struct System {
    bcast: Broadcast,
    launched: FxHashMap<BastionId, Proc<Supervisor>>,
    // TODO: set limit
    restart: FxHashSet<BastionId>,
    waiting: FuturesUnordered<Proc<Supervisor>>,
    pre_start_msgs: Vec<BastionMessage>,
    started: bool,
}

impl System {
    pub(super) fn init() -> Sender {
        let parent = Parent::none();
        let bcast = Broadcast::with_id(parent, NIL_ID);
        let launched = FxHashMap::default();
        let restart = FxHashSet::default();
        let waiting = FuturesUnordered::new();
        let pre_start_msgs = Vec::new();
        let started = false;

        let sender = bcast.sender().clone();

        let system = System {
            bcast,
            launched,
            restart,
            waiting,
            pre_start_msgs,
            started,
        };

        let parent = Parent::system();
        let bcast = Broadcast::with_id(parent, NIL_ID);

        let supervisor = Supervisor::new(bcast);
        let msg = BastionMessage::deploy_supervisor(supervisor);
        system.bcast.send_self(msg);

        Proc::spawn(system.run());

        sender
    }

    // TODO: set a limit?
    async fn recover(&mut self, mut supervisor: Supervisor) {
        let parent = Parent::system();
        let bcast = if supervisor.id() == &NIL_ID {
            Broadcast::with_id(parent, NIL_ID)
        } else {
            Broadcast::new(parent)
        };

        let id = bcast.id().clone();

        supervisor.reset(bcast).await;
        self.bcast.register(supervisor.bcast());

        let launched = Proc::spawn(supervisor.run());
        self.launched.insert(id, launched);
    }

    async fn stop(&mut self) {
        self.bcast.stop_children();

        for (_, launched) in self.launched.drain() {
            self.waiting.push(launched);
        }

        loop {
            match poll!(&mut self.waiting.next()) {
                Poll::Ready(Some(_)) => (),
                Poll::Ready(None) => return,
                Poll::Pending => pending!(),
            }
        }
    }

    async fn kill(&mut self) {
        self.bcast.kill_children();

        for (_, launched) in self.launched.drain() {
            self.waiting.push(launched);
        }

        loop {
            match poll!(&mut self.waiting.next()) {
                Poll::Ready(Some(_)) => (),
                Poll::Ready(None) => return,
                Poll::Pending => pending!(),
            }
        }
    }

    async fn handle(&mut self, msg: BastionMessage) -> Result<(), ()> {
        match msg {
            BastionMessage::Start => unreachable!(),
            BastionMessage::Stop => {
                self.started = false;

                self.stop().await;

                return Err(());
            }
            BastionMessage::PoisonPill => {
                self.started = false;

                self.kill().await;

                return Err(());
            }
            BastionMessage::Deploy(deployment) => match deployment {
                Deployment::Supervisor(supervisor) => {
                    self.bcast.register(supervisor.bcast());

                    let id = supervisor.id().clone();
                    let launched = Proc::spawn(supervisor.run());
                    self.launched.insert(id, launched);
                }
                // FIXME
                Deployment::Children(_) => unimplemented!(),
            },
            BastionMessage::Prune { id } => {
                // TODO: Err if None?
                if let Some(launched) = self.launched.remove(&id) {
                    // TODO: stop or kill?
                    self.bcast.kill_child(&id);

                    self.waiting.push(launched);
                }
            }
            // FIXME
            BastionMessage::SuperviseWith(_) => unimplemented!(),
            BastionMessage::Message(_) => {
                self.bcast.send_children(msg);
            }
            BastionMessage::Dead { id } => {
                // TODO: Err if None?
                if let Some(launched) = self.launched.remove(&id) {
                    self.waiting.push(launched);
                    self.restart.remove(&id);
                }
            }
            BastionMessage::Faulted { id } => {
                // TODO: Err if None?
                if let Some(launched) = self.launched.remove(&id) {
                    self.waiting.push(launched);
                    self.restart.insert(id);
                }
            }
        }

        Ok(())
    }

    async fn run(mut self) {
        loop {
            if let Poll::Ready(Some(supervisor)) = poll!(&mut self.waiting.next()) {
                let id = supervisor.id();
                self.bcast.unregister(&id);

                if self.restart.remove(&id) {
                    self.recover(supervisor).await;
                }

                continue;
            }

            match poll!(&mut self.bcast.next()) {
                // TODO: Err if started == true?
                Poll::Ready(Some(BastionMessage::Start)) => {
                    self.started = true;

                    let msgs = self.pre_start_msgs.drain(..).collect::<Vec<_>>();
                    self.pre_start_msgs.shrink_to_fit();

                    for msg in msgs {
                        // FIXME: Err(Error)?
                        if self.handle(msg).await.is_err() {
                            return;
                        }
                    }

                    let msg = BastionMessage::start();
                    self.bcast.send_children(msg);
                }
                Poll::Ready(Some(msg)) if !self.started => self.pre_start_msgs.push(msg),
                Poll::Ready(Some(msg)) => {
                    // FIXME: Err(Error)?
                    if self.handle(msg).await.is_err() {
                        return;
                    }
                }
                // FIXME
                Poll::Ready(None) => unimplemented!(),
                Poll::Pending => pending!(),
            }
        }
    }
}
