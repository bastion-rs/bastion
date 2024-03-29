//!
//! Child is a element of Children group executing user-defined computation
use crate::broadcast::Broadcast;
use crate::callbacks::{CallbackType, Callbacks};
use crate::child_ref::ChildRef;
use crate::context::{BastionContext, BastionId, ContextState};
use crate::envelope::Envelope;
use crate::message::BastionMessage;
use crate::prelude::ChildrenRef;
#[cfg(feature = "scaling")]
use crate::resizer::ActorGroupStats;
use crate::system::SYSTEM;
use anyhow::Result as AnyResult;

use bastion_executor::pool;
use futures::pending;
use futures::poll;
use futures::prelude::*;
use lightproc::prelude::*;
use lightproc::proc_state::EmptyProcState;
use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tracing::{debug, error, trace, warn};

pub(crate) struct Init(pub(crate) Box<dyn Fn(BastionContext) -> Exec + Send>);
pub(crate) struct Exec(pub(crate) Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>);

#[derive(Debug)]
pub(crate) struct Child {
    bcast: Broadcast,
    // The callbacks called at the group's different lifecycle
    // events.
    callbacks: Callbacks,
    // The future that this child is executing.
    exec: Exec,
    // The child's context state.
    // This is used to store the messages that were received
    // for the child's associated future to be able to
    // retrieve them.
    state: Arc<Pin<Box<ContextState>>>,
    // Messages that were received before the child was
    // started. Those will be "replayed" once a start message
    // is received.
    pre_start_msgs: Vec<Envelope>,
    // A shortcut for accessing to this actor by others.
    child_ref: ChildRef,
    started: bool,
}

impl Init {
    pub(crate) fn new<C, F>(init: C) -> Self
    where
        C: Fn(BastionContext) -> F + Send + 'static,
        F: Future<Output = Result<(), ()>> + Send + 'static,
    {
        let init = Box::new(move |ctx: BastionContext| {
            let fut = init(ctx);
            let exec = Box::pin(fut);

            Exec(exec)
        });

        Init(init)
    }
}

impl Child {
    pub(crate) fn new(
        exec: Exec,
        callbacks: Callbacks,
        bcast: Broadcast,
        state: Arc<Pin<Box<ContextState>>>,
        child_ref: ChildRef,
    ) -> Self {
        debug!("Child({}): Initializing.", bcast.id());
        let pre_start_msgs = Vec::new();
        let started = false;

        Child {
            bcast,
            callbacks,
            exec,
            state,
            pre_start_msgs,
            child_ref,
            started,
        }
    }

    fn stack(&self) -> ProcStack {
        trace!("Child({}): Creating ProcStack.", self.id());
        let id = self.bcast.id().clone();
        // FIXME: panics?
        let parent = self.bcast.parent().clone().into_children().unwrap();
        let path = self.bcast.path().clone();
        let sender = self.bcast.sender().clone();

        let parent_inner = self.bcast.parent().clone().into_children();
        let child_ref_inner = self.child_ref.clone();

        // FIXME: with_pid
        ProcStack::default().with_after_panic(move |_state: &mut EmptyProcState| {
            warn!("Child({}): Panicked.", id);

            if let Some(parent) = &parent_inner {
                Self::remove_from_dispatchers(parent, &child_ref_inner);
                let _ = Self::remove_from_distributors(parent, &child_ref_inner).map_err(|_| {
                    warn!(
                        "Child({}): couldn't remove child from parent distributors.",
                        id
                    )
                });
            }

            let id = id.clone();
            let msg = BastionMessage::restart_required(id, parent.id().clone());
            let env = Envelope::new(msg, path.clone(), sender.clone());
            // TODO: handle errors
            parent.send(env).ok();
        })
    }

    pub(crate) fn id(&self) -> &BastionId {
        self.bcast.id()
    }

    fn stopped(&mut self) {
        debug!("Child({}): Stopped.", self.id());
        let parent = self.bcast.parent().clone().into_children().unwrap();

        Self::remove_from_dispatchers(&parent, &self.child_ref);
        let _ = Self::remove_from_distributors(&parent, &self.child_ref);
        self.bcast.stopped();
    }

    fn faulted(&mut self) {
        debug!("Child({}): Faulted.", self.id());
        let parent = self.bcast.parent().clone().into_children().unwrap();

        Self::remove_from_dispatchers(&parent, &self.child_ref);
        let _ = Self::remove_from_distributors(&parent, &self.child_ref);

        let path = self.bcast.path().clone();
        let sender = self.bcast.sender().clone();

        let msg = BastionMessage::restart_required(self.id().clone(), parent.id().clone());
        let env = Envelope::new(msg, path, sender);
        // TODO: handle errors
        parent.send(env).ok();
    }

    async fn handle(&mut self, env: Envelope) -> Result<(), ()> {
        match env {
            Envelope {
                msg: BastionMessage::Start,
                ..
            } => unreachable!(),
            Envelope {
                msg: BastionMessage::Stop,
                ..
            } => {
                self.stopped();

                #[cfg(feature = "scaling")]
                self.cleanup_actors_stats().await;

                self.callbacks.after_stop();
                return Err(());
            }
            Envelope {
                msg: BastionMessage::Kill,
                ..
            } => {
                self.stopped();

                #[cfg(feature = "scaling")]
                self.cleanup_actors_stats().await;

                self.callbacks.before_restart();
                return Err(());
            }
            // FIXME
            Envelope {
                msg: BastionMessage::Deploy(_),
                ..
            } => unimplemented!(),
            // FIXME
            Envelope {
                msg: BastionMessage::Prune { .. },
                ..
            } => unimplemented!(),
            Envelope {
                msg: BastionMessage::ApplyCallback(callback_type),
                ..
            } => self.apply_callback(callback_type),
            // FIXME
            Envelope {
                msg: BastionMessage::SuperviseWith(_),
                ..
            } => unimplemented!(),
            Envelope {
                msg: BastionMessage::InstantiatedChild { .. },
                ..
            } => unreachable!(),
            Envelope {
                msg: BastionMessage::Message(msg),
                sign,
            } => {
                debug!("Child({}): Received a message: {:?}", self.id(), msg);
                self.state.push_message(msg, sign);
            }
            Envelope {
                msg: BastionMessage::RestartRequired { .. },
                ..
            } => unreachable!(),
            Envelope {
                msg: BastionMessage::RestartSubtree,
                ..
            } => unreachable!(),
            Envelope {
                msg: BastionMessage::RestoreChild { .. },
                ..
            } => unreachable!(),
            Envelope {
                msg: BastionMessage::FinishedChild { .. },
                ..
            } => unreachable!(),
            Envelope {
                msg: BastionMessage::DropChild { .. },
                ..
            } => unreachable!(),
            Envelope {
                msg: BastionMessage::SetState { state },
                ..
            } => {
                debug!("Child({}): Setting new state: {:?}", self.id(), state);
                self.state = state;
            }
            // FIXME
            Envelope {
                msg: BastionMessage::Stopped { .. },
                ..
            } => unimplemented!(),
            // FIXME
            Envelope {
                msg: BastionMessage::Faulted { .. },
                ..
            } => unimplemented!(),
            Envelope {
                msg: BastionMessage::Heartbeat,
                ..
            } => unreachable!(),
        }

        Ok(())
    }

    async fn initialize(&mut self) -> Result<(), ()> {
        trace!(
            "Child({}): Received a new message (started=false): {:?}",
            self.id(),
            BastionMessage::Start
        );
        debug!("Child({}): Starting.", self.id());
        self.callbacks.before_start();
        self.started = true;

        let msgs = self.pre_start_msgs.drain(..).collect::<Vec<_>>();
        self.pre_start_msgs.shrink_to_fit();

        debug!(
            "Child({}): Replaying messages received before starting.",
            self.id()
        );
        for msg in msgs {
            trace!("Child({}): Replaying message: {:?}", self.id(), msg);
            if self.handle(msg).await.is_err() {
                return Err(());
            }
        }

        Ok(())
    }

    fn apply_callback(&mut self, callback_type: CallbackType) {
        match callback_type {
            CallbackType::BeforeStart => self.callbacks.before_start(),
            CallbackType::BeforeRestart => self.callbacks.before_restart(),
            CallbackType::AfterRestart => self.callbacks.after_restart(),
            CallbackType::AfterStop => self.callbacks.after_stop(),
            CallbackType::AfterStart => self.callbacks.after_start(),
        }
    }

    #[cfg(feature = "scaling")]
    async fn update_stats(&mut self) {
        let mailbox_size = self.state.mailbox_size();
        let storage = self.state.stats();

        let mut stats = ActorGroupStats::load(storage.clone());
        stats.update_average_mailbox_size(mailbox_size);
        stats.store(storage);

        let actor_stats_table = self.state.actor_stats();
        actor_stats_table
            .insert(self.bcast.id().clone(), mailbox_size)
            .ok();
    }

    async fn run(mut self) {
        debug!("Child({}): Launched.", self.id());
        let parent = self.bcast.parent().clone().into_children().unwrap();

        if let Err(e) = Self::register_in_dispatchers(&parent, &self.child_ref) {
            error!("couldn't add actor to the registry: {}", e);
            return;
        };
        if let Err(e) = Self::register_to_distributors(&parent, &self.child_ref) {
            error!("couldn't add actor to the distributors: {}", e);
            return;
        };

        self.callbacks.after_start();

        loop {
            #[cfg(feature = "scaling")]
            self.update_stats().await;

            match poll!(&mut self.bcast.next()) {
                // TODO: Err if started == true?
                Poll::Ready(Some(Envelope {
                    msg: BastionMessage::Start,
                    ..
                })) => {
                    if self.initialize().await.is_err() {
                        error!("couldn't initialize Child with id: {}", self.id());
                        return;
                    }

                    continue;
                }
                Poll::Ready(Some(msg)) if !self.started => {
                    trace!(
                        "Child({}): Received a new message (started=false): {:?}",
                        self.id(),
                        msg
                    );
                    self.pre_start_msgs.push(msg);

                    continue;
                }
                Poll::Ready(Some(msg)) => {
                    trace!(
                        "Child({}): Received a new message (started=true): {:?}",
                        self.id(),
                        msg
                    );
                    if self.handle(msg).await.is_err() {
                        error!("Child({}): Couldn't handle message", self.id());
                        return;
                    }

                    continue;
                }
                // NOTE: because `Broadcast` always holds both a `Sender` and
                //      `Receiver` of the same channel, this would only be
                //      possible if the channel was closed, which never happens.
                Poll::Ready(None) => unreachable!(),
                Poll::Pending => (),
            }

            #[cfg(feature = "scaling")]
            self.update_stats().await;

            if !self.started {
                pending!();

                continue;
            }

            match poll!(&mut self.exec) {
                Poll::Ready(Ok(())) => {
                    debug!(
                        "Child({}): The future finished executing successfully.",
                        self.id()
                    );
                    return self.stopped();
                }
                Poll::Ready(Err(())) => {
                    warn!("Child({}): The future returned an error.", self.id());
                    return self.faulted();
                }
                Poll::Pending => (),
            }

            pending!();
        }
    }

    pub(crate) fn launch(self) -> RecoverableHandle<()> {
        let stack = self.stack();
        pool::spawn(self.run(), stack)
    }

    /// Adds the actor into each registry declared in the parent node.
    fn register_in_dispatchers(parent: &ChildrenRef, child_ref: &ChildRef) -> AnyResult<()> {
        let used_dispatchers = parent.dispatchers();

        let global_dispatcher = SYSTEM.dispatcher();
        // FIXME: Pass the module name explicitly?
        let module_name = module_path!().to_string();
        global_dispatcher.register(used_dispatchers, &child_ref, module_name)
    }

    /// Adds the actor into each distributor declared in the parent node.
    fn register_to_distributors(parent: &ChildrenRef, child_ref: &ChildRef) -> AnyResult<()> {
        let distributors = parent.distributors();

        let global_dispatcher = SYSTEM.dispatcher();
        distributors
            .iter()
            .map(|&distributor| {
                global_dispatcher.register_recipient(&distributor, child_ref.clone())
            })
            .collect::<AnyResult<Vec<_>>>()?;

        Ok(())
    }

    /// Cleanup the actor's record from each declared distributor.
    fn remove_from_distributors(parent: &ChildrenRef, child_ref: &ChildRef) -> AnyResult<()> {
        let distributors = parent.distributors();
        let global_dispatcher = SYSTEM.dispatcher();
        global_dispatcher.remove_recipient(distributors, &child_ref)?;
        Ok(())
    }

    /// Cleanup the actor's record from each declared dispatcher.
    fn remove_from_dispatchers(parent: &ChildrenRef, child_ref: &ChildRef) {
        let used_dispatchers = parent.dispatchers();
        let global_dispatcher = SYSTEM.dispatcher();
        global_dispatcher.remove(used_dispatchers, &child_ref);
    }

    #[cfg(feature = "scaling")]
    async fn cleanup_actors_stats(&mut self) {
        self.state
            .actor_stats()
            .remove(&self.bcast.id().clone())
            .ok();
    }
}

impl Future for Exec {
    type Output = Result<(), ()>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        Pin::new(&mut self.get_mut().0).poll(ctx)
    }
}

impl Default for Init {
    fn default() -> Self {
        Init::new(|_| async { Ok(()) })
    }
}

impl Debug for Init {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("Init").finish()
    }
}

impl Debug for Exec {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("Exec").finish()
    }
}
