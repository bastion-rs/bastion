//!
//! Supervisors enable users to supervise a subtree of children
//! or other supervisor trees under themselves.
use crate::broadcast::{Broadcast, Parent, Sender};
use crate::callbacks::Callbacks;
use crate::children::Children;
use crate::children_ref::ChildrenRef;
use crate::context::BastionId;
use crate::envelope::Envelope;
use crate::message::{BastionMessage, Deployment, Message};
use crate::path::{BastionPath, BastionPathElement};
use bastion_executor::pool;
use futures::prelude::*;
use futures::stream::FuturesOrdered;
use futures::{pending, poll};
use futures_timer::Delay;
use fxhash::FxHashMap;
use lightproc::prelude::*;
use log::Level;
use std::cmp::{Eq, PartialEq};
use std::ops::RangeFrom;
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;

#[derive(Debug)]
/// A supervisor that can supervise both [`Children`] and other
/// supervisors using a defined [`SupervisionStrategy`] (set
/// with [`with_strategy`] or [`SupervisionStrategy::OneForOne`]
/// by default).
///
/// When a supervised children group or supervisor faults, the
/// supervisor will restart it and eventually some of its other
/// supervised entities, depending on its supervision strategy.
///
/// Note that a supervisor, called the "system supervisor", is
/// created by the system at startup and is the supervisor
/// supervising children groups created via [`Bastion::children`].
///
/// # Example
///
/// ```rust
/// # use bastion::prelude::*;
/// #
/// # fn main() {
///     # Bastion::init();
///     #
/// let sp_ref: SupervisorRef = Bastion::supervisor(|sp| {
///     // Configure the supervisor...
///     sp.with_strategy(SupervisionStrategy::OneForOne)
///     // ...and return it.
/// }).expect("Couldn't create the supervisor.");
///     #
///     # Bastion::start();
///     # Bastion::stop();
///     # Bastion::block_until_stopped();
/// # }
/// ```
///
/// [`Children`]: children/struct.Children.html
/// [`SupervisionStrategy`]: supervisor/enum.SupervisionStrategy.html
/// [`with_strategy`]: #method.with_strategy
/// [`Bastion::children`]: struct.Bastion.html#method.children
pub struct Supervisor {
    bcast: Broadcast,
    // The order in which children and supervisors were added.
    // It is only updated when at least one of those is resat.
    order: Vec<BastionId>,
    // The currently launched supervised children and supervisors.
    // The last value is the amount of times a given actor has restarted.
    launched: FxHashMap<BastionId, (usize, RecoverableHandle<Supervised>, usize)>,
    // Supervised children and supervisors that are stopped.
    // This is used when resetting or recovering when the
    // supervision strategy is not "one-for-one".
    stopped: FxHashMap<BastionId, Supervised>,
    // Supervised children and supervisors that were killed.
    // This is used when resetting only.
    killed: FxHashMap<BastionId, Supervised>,
    strategy: SupervisionStrategy,
    restart_strategy: ActorRestartStrategy,
    // The callbacks called at the supervisor's different
    // lifecycle events.
    callbacks: Callbacks,
    // Whether this supervisor was started by the system (in
    // which case, users shouldn't be able to get a reference
    // to it).
    is_system_supervisor: bool,
    // Messages that were received before the supervisor was
    // started. Those will be "replayed" once a start message
    // is received.
    pre_start_msgs: Vec<Envelope>,
    started: bool,
}

#[derive(Debug, Clone)]
/// A "reference" to a [`Supervisor`], allowing to
/// communicate with it.
///
/// [`Supervisor`]: supervisor/struct.Supervisor.html
pub struct SupervisorRef {
    id: BastionId,
    sender: Sender,
    path: Arc<BastionPath>,
}

#[derive(Debug, Clone)]
/// The strategy a supervisor should use when one of its
/// supervised children groups or supervisors dies (in
/// the case of a children group, it could be because one
/// of its elements panicked or returned an error).
///
/// The default strategy is `OneForOne`.
pub enum SupervisionStrategy {
    /// When a children group dies (either because it got
    /// killed, it panicked or returned an error), only
    /// this group is restarted.
    OneForOne,
    /// When a children group dies (either because it got
    /// killed, it panicked or returned an error), all the
    /// children groups are restarted (even those which were
    /// stopped) in the same order they were added to the
    /// supervisor.
    OneForAll,
    /// When a children group dies (either because it got
    /// killed, it panicked or returned an error), this
    /// group and all the ones that were added to the
    /// supervisor after it are restarted (even those which
    /// were stopped) in the same order they were added to
    /// the supervisor.
    RestForOne,
}

#[derive(Debug)]
enum Supervised {
    Supervisor(Supervisor),
    Children(Children),
}

#[derive(Debug, Clone)]
/// The strategy for restating an actor as far as it
/// returned an failure.
///
/// The default strategy is `Immediate`.
pub enum ActorRestartStrategy {
    /// Restart an actor as soon as possible, since the moment
    /// the actor finished with a failure.
    Immediate,
    /// Restart an actor after with the timeout. Each next restart
    /// is increasing on the given duration.
    LinearBackOff {
        /// An initial delay before the restarting an actor.
        timeout: Duration,
    },
    /// Restart an actor after with the timeout. Each next timeout
    /// is increasing exponentially.
    /// When passed a multiplier that equals to 1, the strategy works as the
    /// linear back off strategy. Passing the multiplier that equals to 0 leads
    /// to constant restart delays which is equal to the given timeout.
    ExponentialBackOff {
        /// An initial delay before the restarting an actor.
        timeout: Duration,
        /// Defines a multiplier how fast the timeout will be increasing.
        multiplier: u64,
    },
}

impl Supervisor {
    pub(crate) fn new(bcast: Broadcast) -> Self {
        debug!("Supervisor({}): Initializing.", bcast.id());
        let order = Vec::new();
        let launched = FxHashMap::default();
        let stopped = FxHashMap::default();
        let killed = FxHashMap::default();
        let strategy = SupervisionStrategy::default();
        let restart_strategy = ActorRestartStrategy::default();
        let callbacks = Callbacks::new();
        let is_system_supervisor = false;
        let pre_start_msgs = Vec::new();
        let started = false;

        Supervisor {
            bcast,
            order,
            launched,
            stopped,
            killed,
            strategy,
            restart_strategy,
            callbacks,
            is_system_supervisor,
            pre_start_msgs,
            started,
        }
    }

    pub(crate) fn system(bcast: Broadcast) -> Self {
        let mut supervisor = Supervisor::new(bcast);
        supervisor.is_system_supervisor = true;

        supervisor
    }

    fn stack(&self) -> ProcStack {
        trace!("Supervisor({}): Creating ProcStack.", self.id());
        // FIXME: with_pid
        ProcStack::default()
    }

    pub(crate) async fn reset(&mut self, bcast: Option<Broadcast>) {
        if log_enabled!(Level::Debug) {
            if let Some(bcast) = &bcast {
                debug!(
                    "Supervisor({}): Resetting to Supervisor({}).",
                    self.id(),
                    bcast.id()
                );
            } else {
                debug!(
                    "Supervisor({}): Resetting to Supervisor({}).",
                    self.id(),
                    self.id()
                );
            }
        }

        // TODO: stop or kill?
        self.kill(0..).await;

        if let Some(bcast) = bcast {
            self.bcast = bcast;
        } else {
            self.bcast.clear_children();
        }

        debug!(
            "Supervisor({}): Removing {} pre-start messages.",
            self.id(),
            self.pre_start_msgs.len()
        );
        self.pre_start_msgs.clear();
        self.pre_start_msgs.shrink_to_fit();

        self.restart(0..).await;

        debug!(
            "Supervisor({}): Removing {} stopped elements.",
            self.id(),
            self.stopped.len()
        );
        // TODO: should be empty
        self.stopped.clear();
        self.stopped.shrink_to_fit();

        debug!(
            "Supervisor({}): Removing {} killed elements.",
            self.id(),
            self.killed.len()
        );
        // TODO: should be empty
        self.killed.clear();
        self.killed.shrink_to_fit();
    }

    /// Returns this supervisor's identifier.
    ///
    /// Note that the supervisor's identifier is reset when it is
    /// restarted.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    /// Bastion::supervisor(|sp| {
    ///     let supervisor_id: &BastionId = sp.id();
    ///     // ...
    ///     # sp
    /// }).expect("Couldn't create the supervisor.");
    ///
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    pub fn id(&self) -> &BastionId {
        &self.bcast.id()
    }

    pub(crate) fn bcast(&self) -> &Broadcast {
        &self.bcast
    }

    pub(crate) fn callbacks(&self) -> &Callbacks {
        &self.callbacks
    }

    pub(crate) fn as_ref(&self) -> SupervisorRef {
        trace!(
            "Supervisor({}): Creating new SupervisorRef({}).",
            self.id(),
            self.id()
        );
        // TODO: clone or ref?
        let id = self.bcast.id().clone();
        let sender = self.bcast.sender().clone();
        let path = self.bcast.path().clone();

        SupervisorRef::new(id, sender, path)
    }

    /// Creates a new supervisor, passes it through the specified
    /// `init` closure and then starts supervising it.
    ///
    /// If you don't need to chain calls to this `Supervisor`'s methods
    /// and need to get a [`SupervisorRef`] referencing the newly
    /// created supervisor, use the [`supervisor_ref`] method instead.
    ///
    /// # Arguments
    ///
    /// * `init` - The closure taking the new supervisor as an
    ///     argument and returning it once configured.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    ///     # Bastion::supervisor(|parent| {
    /// parent.supervisor(|sp| {
    ///     // Configure the supervisor...
    ///     sp.with_strategy(SupervisionStrategy::OneForOne)
    ///     // ...and return it.
    /// })
    ///     # }).unwrap();
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    ///
    /// [`SupervisorRef`]: ../struct.SupervisorRef.html
    /// [`supervisor_ref`]: #method.supervisor_ref
    pub fn supervisor<S>(self, init: S) -> Self
    where
        S: FnOnce(Supervisor) -> Supervisor,
    {
        debug!("Supervisor({}): Creating supervisor.", self.id());
        let parent = Parent::supervisor(self.as_ref());
        let bcast = Broadcast::new(parent, BastionPathElement::Supervisor(BastionId::new()));

        debug!(
            "Supervisor({}): Initializing Supervisor({}).",
            self.id(),
            bcast.id()
        );
        let supervisor = Supervisor::new(bcast);
        let supervisor = init(supervisor);
        debug!("Supervisor({}): Initialized.", supervisor.id());

        debug!(
            "Supervisor({}): Deploying Supervisor({}).",
            self.id(),
            supervisor.id()
        );
        let msg = BastionMessage::deploy_supervisor(supervisor);
        let env = Envelope::new(msg, self.bcast.path().clone(), self.bcast.sender().clone());
        self.bcast.send_self(env);

        self
    }

    /// Creates a new `Supervisor`, passes it through the specified
    /// `init` closure and then starts supervising it.
    ///
    /// If you need to chain calls to this `Supervisor`'s methods and
    /// don't need to get a [`SupervisorRef`] referencing the newly
    /// created supervisor, use the [`supervisor`] method instead.
    ///
    /// # Arguments
    ///
    /// * `init` - The closure taking the new `Supervisor` as an
    ///     argument and returning it once configured.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    ///     # Bastion::supervisor(|mut parent| {
    /// let sp_ref: SupervisorRef = parent.supervisor_ref(|sp| {
    ///     // Configure the supervisor...
    ///     sp.with_strategy(SupervisionStrategy::OneForOne)
    ///     // ...and return it.
    /// });
    ///         # parent
    ///     # }).unwrap();
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    ///
    /// [`SupervisorRef`]: ../struct.SupervisorRef.html
    /// [`supervisor`]: #method.supervisor
    pub fn supervisor_ref<S>(&mut self, init: S) -> SupervisorRef
    where
        S: FnOnce(Supervisor) -> Supervisor,
    {
        debug!("Supervisor({}): Creating supervisor.", self.id());
        let parent = Parent::supervisor(self.as_ref());
        let bcast = Broadcast::new(parent, BastionPathElement::Supervisor(BastionId::new()));

        debug!(
            "Supervisor({}): Initializing Supervisor({}).",
            self.id(),
            bcast.id()
        );
        let supervisor = Supervisor::new(bcast);
        let supervisor = init(supervisor);
        debug!("Supervisor({}): Initialized.", supervisor.id());
        let supervisor_ref = supervisor.as_ref();

        debug!(
            "Supervisor({}): Deploying Supervisor({}).",
            self.id(),
            supervisor.id()
        );
        let msg = BastionMessage::deploy_supervisor(supervisor);
        let env = Envelope::new(msg, self.bcast.path().clone(), self.bcast.sender().clone());
        self.bcast.send_self(env);

        supervisor_ref
    }

    /// Creates a new [`Children`], passes it through the specified
    /// `init` closure and then starts supervising it.
    ///
    /// If you don't need to chain calls to this `Supervisor`'s methods
    /// and need to get a [`ChildrenRef`] referencing the newly
    /// created supervisor, use the [`children`] method instead.
    ///
    /// # Arguments
    ///
    /// * `init` - The closure taking the new `Children` as an
    ///     argument and returning it once configured.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    ///     # Bastion::supervisor(|sp| {
    /// sp.children(|children| {
    ///     children.with_exec(|ctx: BastionContext| {
    ///         async move {
    ///             // Send and receive messages...
    ///             let opt_msg: Option<SignedMessage> = ctx.try_recv().await;
    ///
    ///             // ...and return `Ok(())` or `Err(())` when you are done...
    ///             Ok(())
    ///             // Note that if `Err(())` was returned, the supervisor would
    ///             // restart the children group.
    ///         }
    ///     })
    /// })
    ///     # }).unwrap();
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    ///
    /// [`Children`]: children/struct.Children.html
    /// [`ChildrenRef`]: children/struct.ChildrenRef.html
    /// [`children_ref`]: #method.children_ref
    pub fn children<C>(self, init: C) -> Self
    where
        C: FnOnce(Children) -> Children,
    {
        debug!("Supervisor({}): Creating children group.", self.id());
        let parent = Parent::supervisor(self.as_ref());
        let bcast = Broadcast::new(parent, BastionPathElement::Children(BastionId::new()));

        debug!(
            "Supervisor({}): Initializing Children({}).",
            self.id(),
            bcast.id()
        );
        let children = Children::new(bcast);
        let mut children = init(children);
        debug!("Children({}): Initialized.", children.id());
        // FIXME: children group elems launched without the group itself being launched
        children.launch_elems();

        debug!(
            "Supervisor({}): Deploying Children({}).",
            self.id(),
            children.id()
        );
        let msg = BastionMessage::deploy_children(children);
        let env = Envelope::new(msg, self.bcast.path().clone(), self.bcast.sender().clone());
        self.bcast.send_self(env);

        self
    }

    /// Creates a new [`Children`], passes it through the specified
    /// `init` closure and then starts supervising it.
    ///
    /// If you need to chain calls to this `Supervisor`'s methods and
    /// don't need to get a [`ChildrenRef`] referencing the newly
    /// created supervisor, use the [`children`] method instead.
    ///
    /// # Arguments
    ///
    /// * `init` - The closure taking the new `Children` as an
    ///     argument and returning it once configured.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    ///     # Bastion::supervisor(|mut sp| {
    /// let children_ref: ChildrenRef = sp.children_ref(|children| {
    ///     children.with_exec(|ctx: BastionContext| {
    ///         async move {
    ///             // Send and receive messages...
    ///             let opt_msg: Option<SignedMessage> = ctx.try_recv().await;
    ///
    ///             // ...and return `Ok(())` or `Err(())` when you are done...
    ///             Ok(())
    ///             // Note that if `Err(())` was returned, the supervisor would
    ///             // restart the children group.
    ///         }
    ///     })
    /// });
    ///         # sp
    ///     # }).unwrap();
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    ///
    /// [`Children`]: children/struct.Children.html
    /// [`ChildrenRef`]: children/struct.ChildrenRef.html
    /// [`children`]: #method.children
    pub fn children_ref<C>(&self, init: C) -> ChildrenRef
    where
        C: FnOnce(Children) -> Children,
    {
        debug!("Supervisor({}): Creating children group.", self.id());
        let parent = Parent::supervisor(self.as_ref());
        let bcast = Broadcast::new(parent, BastionPathElement::Children(BastionId::new()));

        debug!(
            "Supervisor({}): Initializing Children({}).",
            self.id(),
            bcast.id()
        );
        let children = Children::new(bcast);
        let mut children = init(children);
        debug!("Children({}): Initialized.", children.id());
        // FIXME: children group elems launched without the group itself being launched
        children.launch_elems();

        let children_ref = children.as_ref();
        debug!(
            "Supervisor({}): Deploying Children({}).",
            self.id(),
            children.id()
        );
        let msg = BastionMessage::deploy_children(children);
        let env = Envelope::new(msg, self.bcast.path().clone(), self.bcast.sender().clone());
        self.bcast.send_self(env);

        children_ref
    }

    /// Sets the strategy the supervisor should use when one
    /// of its supervised children groups or supervisors dies
    /// (in the case of a children group, it could be because one
    /// of its elements panicked or returned an error).
    ///
    /// The default strategy is
    /// [`SupervisionStrategy::OneForOne`].
    ///
    /// # Arguments
    ///
    /// * `strategy` - The strategy to use:
    ///     - [`SupervisionStrategy::OneForOne`] would only restart
    ///         the supervised children groups or supervisors that
    ///         fault.
    ///     - [`SupervisionStrategy::OneForAll`] would restart all
    ///         the supervised children groups or supervisors (even
    ///         those which were stopped) when one of them faults,
    ///         respecting the order in which they were added.
    ///     - [`SupervisionStrategy::RestForOne`] would restart the
    ///         supervised children groups or supervisors that fault
    ///         along with all the other supervised children groups
    ///         or supervisors that were added after them (even the
    ///         stopped ones), respecting the order in which they
    ///         were added.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    /// Bastion::supervisor(|sp| {
    ///     // Note that "one-for-one" is the default strategy.
    ///     sp.with_strategy(SupervisionStrategy::OneForOne)
    /// }).expect("Couldn't create the supervisor");
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    ///
    /// [`SupervisionStrategy::OneForOne`]: supervisor/enum.SupervisionStrategy.html#variant.OneForOne
    /// [`SupervisionStrategy::OneForAll`]: supervisor/enum.SupervisionStrategy.html#variant.OneForAll
    /// [`SupervisionStrategy::RestForOne`]: supervisor/enum.SupervisionStrategy.html#variant.RestForOne
    pub fn with_strategy(mut self, strategy: SupervisionStrategy) -> Self {
        trace!(
            "Supervisor({}): Setting strategy: {:?}",
            self.id(),
            strategy
        );
        self.strategy = strategy;
        self
    }

    /// Sets the actor restart strategy the supervisor should use
    /// of its supervised children groups or supervisors dies to
    /// restore in the correct state.
    ///
    /// The default strategy is
    /// [`ActorRestartStrategy::Instanly`].
    ///
    /// # Arguments
    ///
    /// * `restart_strategy` - The strategy to use:
    ///     - [`ActorRestartStrategy::Instantly`] would restart the
    ///         failed actor as soon as possible.
    ///     - [`ActorRestartStrategy::LinearBackOff`] would restart the
    ///         failed actor with the delay increasing linearly.
    ///     - [`ActorRestartStrategy::ExponentialBackOff`] would restart the
    ///         failed actor with the delay, multiplied by given coefficient.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    /// Bastion::supervisor(|sp| {
    ///     sp.with_restart_strategy(ActorRestartStrategy::Immediate)
    /// }).expect("Couldn't create the supervisor");
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    ///
    /// [`ActorRestartStrategy::Instantly`]: supervisor/enum.ActorRestartStrategy.html#variant.Instantly
    /// [`ActorRestartStrategy::LinearBackOff`]: supervisor/enum.ActorRestartStrategy.html#variant.LinearBackOff
    /// [`ActorRestartStrategy::ExponentialBackOff`]: supervisor/enum.ActorRestartStrategy.html#variant.ExponentialBackOff
    pub fn with_restart_strategy(mut self, restart_strategy: ActorRestartStrategy) -> Self {
        trace!(
            "Supervisor({}): Setting actor restart strategy: {:?}",
            self.id(),
            restart_strategy
        );
        self.restart_strategy = restart_strategy;
        self
    }

    /// Sets the callbacks that will get called at this supervisor's
    /// different lifecycle events.
    ///
    /// See [`Callbacks`]'s documentation for more information about the
    /// different callbacks available.
    ///
    /// # Arguments
    ///
    /// * `callbacks` - The callbacks that will get called for this
    ///     supervisor.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    /// Bastion::supervisor(|sp| {
    ///     let callbacks = Callbacks::new()
    ///         .with_before_start(|| println!("Supervisor started."))
    ///         .with_after_stop(|| println!("Supervisor stopped."));
    ///
    ///     sp.with_callbacks(callbacks)
    /// }).expect("Couldn't create the supervisor.");
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    ///
    /// [`Callbacks`]: struct.Callbacks.html
    pub fn with_callbacks(mut self, callbacks: Callbacks) -> Self {
        trace!(
            "Supervisor({}): Setting callbacks: {:?}",
            self.id(),
            callbacks
        );
        self.callbacks = callbacks;
        self
    }

    async fn restart(&mut self, range: RangeFrom<usize>) {
        debug!("Supervisor({}): Restarting range: {:?}", self.id(), range);
        // TODO: stop or kill?
        self.kill(range.clone()).await;

        let restart_strategy = self.restart_strategy.clone();
        let supervisor_id = &self.id().clone();
        let parent = Parent::supervisor(self.as_ref());
        let mut reset = FuturesOrdered::new();
        for id in self.order.drain(range) {
            let (killed, supervised) = if let Some(supervised) = self.stopped.remove(&id) {
                (false, supervised)
            } else if let Some(supervised) = self.killed.remove(&id) {
                (true, supervised)
            } else {
                // FIXME
                unimplemented!();
            };

            if killed {
                supervised.callbacks().before_restart();
            }

            let bcast = Broadcast::new(
                parent.clone(),
                supervised.elem().clone().with_id(BastionId::new()),
            );

            let restart_count = match self.launched.get(&id.clone()) {
                Some((_, _, count)) => *count,
                None => 0,
            };

            let restart_strategy_inner = restart_strategy.clone();
            reset.push(async move {
                debug!(
                    "Supervisor({}): Resetting Supervised({}) (killed={}) to Supervised({}).",
                    supervisor_id,
                    supervised.id(),
                    killed,
                    bcast.id()
                );

                match restart_strategy_inner {
                    ActorRestartStrategy::LinearBackOff { timeout } => {
                        let start_in =
                            timeout.as_secs() + (timeout.as_secs() * restart_count as u64);
                        Delay::new(Duration::from_secs(start_in)).await;
                    }
                    ActorRestartStrategy::ExponentialBackOff {
                        timeout,
                        multiplier,
                    } => {
                        let start_in = timeout.as_secs()
                            + (timeout.as_secs() * multiplier * restart_count as u64);
                        Delay::new(Duration::from_secs(start_in)).await;
                    }
                    _ => {}
                };

                // FIXME: panics?
                let supervised = supervised.reset(bcast).await.unwrap();
                // FIXME: might not keep order
                if killed {
                    supervised.callbacks().after_restart();
                } else {
                    supervised.callbacks().before_start();
                }

                supervised
            })
        }

        trace!(
            "Supervisor({}): Resetting {} elements.",
            self.id(),
            reset.len()
        );
        while let Some(supervised) = reset.next().await {
            self.bcast.register(supervised.bcast());
            if self.started {
                let msg = BastionMessage::start();
                let env =
                    Envelope::new(msg, self.bcast.path().clone(), self.bcast.sender().clone());
                self.bcast.send_child(supervised.id(), env);
            }

            debug!(
                "Supervisor({}): Launching Supervised({}).",
                self.id(),
                supervised.id()
            );
            let id = supervised.id().clone();
            let restart_count = match self.launched.get(&id.clone()) {
                Some((_, _, count)) => *count,
                None => 0,
            };
            let launched = supervised.launch();
            self.launched
                .insert(id.clone(), (self.order.len(), launched, restart_count));
            self.order.push(id);
        }
    }

    async fn stop(&mut self, range: RangeFrom<usize>) {
        debug!("Supervisor({}): Stopping range: {:?}", self.id(), range);
        if range.start == 0 {
            self.bcast.stop_children();
        } else {
            // FIXME: panics
            for id in self.order.get(range.clone()).unwrap() {
                trace!("Supervised({}): Stopping Supervised({}).", self.id(), id);
                self.bcast.stop_child(id);
            }
        }

        let mut supervised = FuturesOrdered::new();
        // FIXME: panics?
        for id in self.order.get(range.clone()).unwrap() {
            // TODO: Err if None?
            if let Some((_, launched, _)) = self.launched.remove(&id) {
                // TODO: add a "stopped" list and poll from it instead of awaiting
                supervised.push(launched);
            }
        }

        while let Some(supervised) = supervised.next().await {
            match supervised {
                Some(supervised) => {
                    trace!(
                        "Supervisor({}): Supervised({}) stopped.",
                        self.id(),
                        supervised.id()
                    );
                    supervised.callbacks().after_stop();

                    let id = supervised.id().clone();
                    self.stopped.insert(id, supervised);
                }
                // FIXME
                None => unimplemented!(),
            }
        }
    }

    async fn kill(&mut self, range: RangeFrom<usize>) {
        debug!("Supervisor({}): Killing range: {:?}", self.id(), range);
        if range.start == 0 {
            self.bcast.kill_children();
        } else {
            // FIXME: panics
            for id in self.order.get(range.clone()).unwrap() {
                trace!("Supervised({}): Killing Supervised({}).", self.id(), id);
                self.bcast.kill_child(id);
            }
        }

        let mut supervised = FuturesOrdered::new();
        // FIXME: panics?
        for id in self.order.get(range.clone()).unwrap() {
            // TODO: Err if None?
            if let Some((_, launched, _)) = self.launched.remove(&id) {
                // TODO: add a "stopped" list and poll from it instead of awaiting
                supervised.push(launched);
            }
        }

        while let Some(supervised) = supervised.next().await {
            match supervised {
                Some(supervised) => {
                    trace!(
                        "Supervisor({}): Supervised({}) stopped.",
                        self.id(),
                        supervised.id()
                    );
                    let id = supervised.id().clone();
                    self.killed.insert(id, supervised);
                }
                // FIXME
                None => unimplemented!(),
            }
        }
    }

    fn stopped(&mut self) {
        debug!("Supervisor({}): Stopped.", self.id());
        self.bcast.stopped();
    }

    fn faulted(&mut self) {
        debug!("Supervisor({}): Faulted.", self.id());
        self.bcast.faulted();
    }

    async fn recover(&mut self, id: BastionId) -> Result<(), ()> {
        debug!(
            "Supervisor({}): Recovering using strategy: {:?}",
            self.id(),
            self.strategy
        );
        match self.strategy {
            SupervisionStrategy::OneForOne => {
                let (order, launched, _) = self.launched.remove(&id).ok_or(())?;
                // TODO: add a "waiting" list and poll from it instead of awaiting
                // FIXME: panics?
                let supervised = launched.await.unwrap();
                supervised.callbacks().before_restart();

                self.bcast.unregister(supervised.id());

                let parent = Parent::supervisor(self.as_ref());
                let bcast =
                    Broadcast::new(parent, supervised.elem().clone().with_id(BastionId::new()));
                let id = bcast.id().clone();
                debug!(
                    "Supervisor({}): Resetting Supervised({}) to Supervised({}).",
                    self.id(),
                    supervised.id(),
                    bcast.id()
                );
                // FIXME: panics?
                let supervised = supervised.reset(bcast).await.unwrap();
                supervised.callbacks().after_restart();

                self.bcast.register(supervised.bcast());
                if self.started {
                    let msg = BastionMessage::start();
                    let env =
                        Envelope::new(msg, self.bcast.path().clone(), self.bcast.sender().clone());
                    self.bcast.send_child(&id, env);
                }

                debug!(
                    "Supervisor({}): Launching Supervised({}).",
                    self.id(),
                    supervised.id()
                );
                let restart_count = match self.launched.get(&id.clone()) {
                    Some((_, _, count)) => count + 1,
                    None => 0,
                };
                let launched = supervised.launch();
                self.launched
                    .insert(id.clone(), (order, launched, restart_count));
                self.order[order] = id;
            }
            SupervisionStrategy::OneForAll => {
                self.restart(0..).await;

                // TODO: should be empty
                self.stopped.shrink_to_fit();
                self.killed.shrink_to_fit();
            }
            SupervisionStrategy::RestForOne => {
                let (start, _, _) = self.launched.get(&id).ok_or(())?;
                let start = *start;

                self.restart(start..).await;
            }
        }

        Ok(())
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
                self.stop(0..).await;
                self.stopped();

                return Err(());
            }
            Envelope {
                msg: BastionMessage::Kill,
                ..
            } => {
                self.kill(0..).await;
                self.stopped();

                return Err(());
            }
            Envelope {
                msg: BastionMessage::Deploy(deployment),
                ..
            } => {
                let supervised = match deployment {
                    Deployment::Supervisor(supervisor) => {
                        debug!(
                            "Supervisor({}): Deploying Supervisor({}).",
                            self.id(),
                            supervisor.id()
                        );
                        supervisor.callbacks().before_start();
                        Supervised::supervisor(supervisor)
                    }
                    Deployment::Children(children) => {
                        debug!(
                            "Supervisor({}): Deploying Children({}).",
                            self.id(),
                            children.id()
                        );
                        children.callbacks().before_start();
                        Supervised::children(children)
                    }
                };

                self.bcast.register(supervised.bcast());
                if self.started {
                    let msg = BastionMessage::start();
                    let env =
                        Envelope::new(msg, self.bcast.path().clone(), self.bcast.sender().clone());
                    self.bcast.send_child(supervised.id(), env);
                }

                debug!(
                    "Supervisor({}): Launching Supervised({}).",
                    self.id(),
                    supervised.id()
                );
                let id = supervised.id().clone();
                let launched = supervised.launch();
                self.launched
                    .insert(id.clone(), (self.order.len(), launched, 0));
                self.order.push(id);
            }
            // FIXME
            Envelope {
                msg: BastionMessage::Prune { .. },
                ..
            } => unimplemented!(),
            Envelope {
                msg: BastionMessage::SuperviseWith(strategy),
                ..
            } => {
                debug!(
                    "Supervisor({}): Setting strategy: {:?}",
                    self.id(),
                    strategy
                );
                self.strategy = strategy;
            }
            Envelope {
                msg: BastionMessage::Message(ref message),
                ..
            } => {
                debug!(
                    "Supervisor({}): Broadcasting a message: {:?}",
                    self.id(),
                    message
                );
                self.bcast.send_children(env);
            }
            Envelope {
                msg: BastionMessage::Stopped { id },
                ..
            } => {
                // FIXME: Err if None?
                if let Some((_, launched, _)) = self.launched.remove(&id) {
                    debug!("Supervisor({}): Supervised({}) stopped.", self.id(), id);
                    // TODO: add a "waiting" list an poll from it instead of awaiting
                    // FIXME: panics?
                    let supervised = launched.await.unwrap();
                    supervised.callbacks().after_stop();

                    self.bcast.unregister(&id);
                    self.stopped.insert(id, supervised);
                }
            }
            Envelope {
                msg: BastionMessage::Faulted { id },
                ..
            } => {
                if self.launched.contains_key(&id) {
                    warn!("Supervisor({}): Supervised({}) faulted.", self.id(), id);
                }

                if self.recover(id).await.is_err() {
                    // TODO: stop or kill?
                    self.kill(0..).await;
                    self.faulted();

                    return Err(());
                }
            }
        }

        Ok(())
    }

    async fn run(mut self) -> Self {
        debug!("Supervisor({}): Launched.", self.id());
        loop {
            match poll!(&mut self.bcast.next()) {
                // TODO: Err if started == true?
                Poll::Ready(Some(Envelope {
                    msg: BastionMessage::Start,
                    ..
                })) => {
                    trace!(
                        "Supervisor({}): Received a new message (started=false): {:?}",
                        self.id(),
                        BastionMessage::Start
                    );
                    debug!("Supervisor({}): Starting.", self.id());
                    self.started = true;

                    let msg = BastionMessage::start();
                    let env =
                        Envelope::new(msg, self.bcast.path().clone(), self.bcast.sender().clone());
                    self.bcast.send_children(env);

                    let msgs = self.pre_start_msgs.drain(..).collect::<Vec<_>>();
                    self.pre_start_msgs.shrink_to_fit();

                    debug!(
                        "Supervisor({}): Replaying messages received before starting.",
                        self.id()
                    );
                    for msg in msgs {
                        trace!("Supervisor({}): Replaying message: {:?}", self.id(), msg);
                        if self.handle(msg).await.is_err() {
                            return self;
                        }
                    }
                }
                Poll::Ready(Some(msg)) if !self.started => {
                    trace!(
                        "Supervisor({}): Received a new message (started=false): {:?}",
                        self.id(),
                        msg
                    );
                    self.pre_start_msgs.push(msg);
                }
                Poll::Ready(Some(msg)) => {
                    trace!(
                        "Supervisor({}): Received a new message (started=true): {:?}",
                        self.id(),
                        msg
                    );
                    if self.handle(msg).await.is_err() {
                        return self;
                    }
                }
                // NOTE: because `Broadcast` always holds both a `Sender` and
                //      `Receiver` of the same channel, this would only be
                //      possible if the channel was closed, which never happens.
                Poll::Ready(None) => unreachable!(),
                Poll::Pending => pending!(),
            }
        }
    }

    pub(crate) fn launch(self) -> RecoverableHandle<Self> {
        debug!("Supervisor({}): Launching.", self.id());
        let stack = self.stack();
        pool::spawn(self.run(), stack)
    }
}

impl SupervisorRef {
    pub(crate) fn new(id: BastionId, sender: Sender, path: Arc<BastionPath>) -> Self {
        SupervisorRef { id, sender, path }
    }

    /// Returns the identifier of the supervisor this `SupervisorRef`
    /// is referencing.
    ///
    /// Note that the supervisor's identifier is reset when it is
    /// restarted.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    /// let supervisor_ref = Bastion::supervisor(|sp| {
    ///     // ...
    ///     # sp
    /// }).expect("Couldn't create the supervisor.");
    ///
    /// let supervisor_id: &BastionId = supervisor_ref.id();
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    pub fn id(&self) -> &BastionId {
        &self.id
    }

    /// Creates a new [`Supervisor`], passes it through the specified
    /// `init` closure and then sends it to the supervisor this
    /// `SupervisorRef` is referencing to supervise it.
    ///
    /// This method returns a [`SupervisorRef`] referencing the newly
    /// created supervisor if it succeeded, or `Err(())`
    /// otherwise.
    ///
    /// # Arguments
    ///
    /// * `init` - The closure taking the new [`Supervisor`] as an
    ///     argument and returning it once configured.
    ///
    /// # Example
    ///
    /// ```
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    ///     # let mut parent_ref = Bastion::supervisor(|sp| sp).unwrap();
    /// let sp_ref: SupervisorRef = parent_ref.supervisor(|sp| {
    ///     // Configure the supervisor...
    ///     sp.with_strategy(SupervisionStrategy::OneForOne)
    ///     // ...and return it.
    /// }).expect("Couldn't create the supervisor.");
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    ///
    /// [`Supervisor`]: supervisor/struct.Supervisor.html
    pub fn supervisor<S>(&self, init: S) -> Result<Self, ()>
    where
        S: FnOnce(Supervisor) -> Supervisor,
    {
        debug!("SupervisorRef({}): Creating supervisor.", self.id());
        let parent = Parent::supervisor(self.clone());
        let bcast = Broadcast::new(parent, BastionPathElement::Supervisor(BastionId::new()));

        debug!(
            "SupervisorRef({}): Initializing Supervisor({}).",
            self.id(),
            bcast.id()
        );
        let supervisor = Supervisor::new(bcast);
        let supervisor = init(supervisor);
        let supervisor_ref = supervisor.as_ref();
        debug!("Supervisor({}): Initialized.", supervisor.id());

        debug!(
            "SupervisorRef({}): Deploying Supervisor({}).",
            self.id(),
            supervisor.id()
        );
        let msg = BastionMessage::deploy_supervisor(supervisor);
        let env = Envelope::new(msg, self.path.clone(), self.sender.clone());
        self.send(env).map_err(|_| ())?;

        Ok(supervisor_ref)
    }

    /// Creates a new [`Children`], passes it through the specified
    /// `init` closure and then sends it to the supervisor this
    /// `SupervisorRef` is referencing to supervise it.
    ///
    /// This methods returns a [`ChildrenRef`] referencing the newly
    /// created children group it it succeeded, or `Err(())`
    /// otherwise.
    ///
    /// # Arguments
    ///
    /// * `init` - The closure taking the new [`Children`] as an
    ///     argument and returning it once configured.
    ///
    /// # Example
    ///
    /// ```
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    ///     # let sp_ref = Bastion::supervisor(|sp| sp).unwrap();
    /// let children_ref: ChildrenRef = sp_ref.children(|children| {
    ///     children.with_exec(|ctx: BastionContext| {
    ///         async move {
    ///             // Send and receive messages...
    ///             let opt_msg: Option<SignedMessage> = ctx.try_recv().await;
    ///
    ///             // ...and return `Ok(())` or `Err(())` when you are done...
    ///             Ok(())
    ///             // Note that if `Err(())` was returned, the supervisor would
    ///             // restart the children group.
    ///         }
    ///     })
    /// }).expect("Couldn't create the children group.");
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    ///
    /// [`Children`]: children/struct.Children.html
    /// [`ChildrenRef`]: children/struct.ChildrenRef.html
    pub fn children<C>(&self, init: C) -> Result<ChildrenRef, ()>
    where
        C: FnOnce(Children) -> Children,
    {
        self.children_with_id(BastionId::new(), init)
    }

    pub(crate) fn children_with_id<C>(&self, id: BastionId, init: C) -> Result<ChildrenRef, ()>
    where
        C: FnOnce(Children) -> Children,
    {
        debug!("SupervisorRef({}): Creating children group.", self.id());
        let parent = Parent::supervisor(self.clone());
        let bcast = Broadcast::new(parent, BastionPathElement::Children(id));

        debug!(
            "SupervisorRef({}): Initializing Children({}).",
            self.id(),
            bcast.id()
        );
        let children = Children::new(bcast);
        let mut children = init(children);
        debug!("Children({}): Initialized.", children.id());
        // FIXME: children group elems launched without the group itself being launched
        children.launch_elems();

        let children_ref = children.as_ref();
        debug!(
            "SupervisorRef({}): Deplying Children({}).",
            self.id(),
            children.id()
        );
        let msg = BastionMessage::deploy_children(children);
        let env = Envelope::new(msg, self.path.clone(), self.sender.clone());
        self.send(env).map_err(|_| ())?;

        Ok(children_ref)
    }

    /// Sends to the supervisor this `SupervisorRef` is
    /// referencing the strategy that it should start
    /// using when one of its supervised children groups or
    /// supervisors dies (in the case of a children group,
    /// it could be because one of its elements panicked or
    /// returned an error).
    ///
    /// The default strategy `Supervisor` is
    /// [`SupervisionStrategy::OneForOne`].
    ///
    /// This method returns `()` if it succeeded, or `Err(())`
    /// otherwise.
    ///
    /// # Arguments
    ///
    /// * `strategy` - The strategy to use:
    ///     - [`SupervisionStrategy::OneForOne`] would only restart
    ///         the supervised children groups or supervisors that
    ///         fault.
    ///     - [`SupervisionStrategy::OneForAll`] would restart all
    ///         the supervised children groups or supervisors (even
    ///         those which were stopped) when one of them faults,
    ///         respecting the order in which they were added.
    ///     - [`SupervisionStrategy::RestForOne`] would restart the
    ///         supervised children groups or supervisors that fault
    ///         along with all the other supervised children groups
    ///         or supervisors that were added after them (even the
    ///         stopped ones), respecting the order in which they
    ///         were added.
    ///
    /// # Example
    ///
    /// ```
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    ///     # let sp_ref = Bastion::supervisor(|sp| sp).unwrap();
    /// // Note that "one-for-one" is the default strategy.
    /// sp_ref.strategy(SupervisionStrategy::OneForOne);
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    ///
    /// [`SupervisionStrategy::OneForOne`]: supervisor/enum.SupervisionStrategy.html#variant.OneForOne
    /// [`SupervisionStrategy::OneForAll`]: supervisor/enum.SupervisionStrategy.html#variant.OneForAll
    /// [`SupervisionStrategy::RestForOne`]: supervisor/enum.SupervisionStrategy.html#variant.RestForOne
    pub fn strategy(&self, strategy: SupervisionStrategy) -> Result<(), ()> {
        debug!(
            "SupervisorRef({}): Setting strategy: {:?}",
            self.id(),
            strategy
        );
        let msg = BastionMessage::supervise_with(strategy);
        let env = Envelope::from_dead_letters(msg);
        self.send(env).map_err(|_| ())
    }

    /// Sends a message to the supervisor this `SupervisorRef`
    /// is referencing which will then send it to all of its
    /// supervised children groups and supervisors.
    ///
    /// This method returns `()` if it succeeded, or `Err(msg)`
    /// otherwise.
    ///
    /// # Arguments
    ///
    /// * `msg` - The message to send.
    ///
    /// # Example
    ///
    /// ```
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    ///     # let sp_ref = Bastion::supervisor(|sp| sp).unwrap();
    /// let msg = "A message containing data.";
    /// sp_ref.broadcast(msg).expect("Couldn't send the message.");
    ///
    ///     # Bastion::children(|children| {
    ///         # children.with_exec(|ctx: BastionContext| {
    ///             # async move {
    /// // And then in every future of the elements of the children
    /// // groups that are supervised by this supervisor or one of
    /// // its supervised supervisors (etc.)...
    /// msg! { ctx.recv().await?,
    ///     ref msg: &'static str => {
    ///         assert_eq!(msg, &"A message containing data.");
    ///     };
    ///     // We are only broadcasting a `&'static str` in this
    ///     // example, so we know that this won't happen...
    ///     _: _ => ();
    /// }
    ///                 #
    ///                 # Ok(())
    ///             # }
    ///         # })
    ///     # }).unwrap();
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    pub fn broadcast<M: Message>(&self, msg: M) -> Result<(), M> {
        debug!(
            "SupervisorRef({}): Broadcasting message: {:?}",
            self.id(),
            msg
        );
        let msg = BastionMessage::broadcast(msg);
        let env = Envelope::from_dead_letters(msg);
        // FIXME: panics?
        self.send(env).map_err(|env| env.into_msg().unwrap())
    }

    /// Sends a message to the supervisor this `SupervisorRef`
    /// is referencing to tell it to stop every running children
    /// groups and supervisors that it is supervising.
    ///
    /// This method returns `()` if it succeeded, or `Err(())`
    /// otherwise.
    ///
    /// # Example
    ///
    /// ```
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    ///     # let sp_ref = Bastion::supervisor(|sp| sp).unwrap();
    /// sp_ref.stop().expect("Couldn't send the message.");
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    pub fn stop(&self) -> Result<(), ()> {
        debug!("SupervisorRef({}): Stopping.", self.id());
        let msg = BastionMessage::stop();
        let env = Envelope::from_dead_letters(msg);
        self.send(env).map_err(|_| ())
    }

    /// Sends a message to the supervisor this `SupervisorRef`
    /// is referencing to tell it to kill every running children
    /// groups and supervisors that it is supervising.
    ///
    /// This method returns `()` if it succeeded, or `Err(())`
    /// otherwise.
    ///
    /// # Example
    ///
    /// ```
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    ///     # let sp_ref = Bastion::supervisor(|sp| sp).unwrap();
    /// sp_ref.kill().expect("Couldn't send the message.");
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    pub fn kill(&self) -> Result<(), ()> {
        debug!("SupervisorRef({}): Killing.", self.id());
        let msg = BastionMessage::kill();
        let env = Envelope::from_dead_letters(msg);
        self.send(env).map_err(|_| ())
    }

    pub(crate) fn send(&self, env: Envelope) -> Result<(), Envelope> {
        trace!("SupervisorRef({}): Sending message: {:?}", self.id(), env);
        self.sender
            .unbounded_send(env)
            .map_err(|err| err.into_inner())
    }

    pub(crate) fn path(&self) -> &Arc<BastionPath> {
        &self.path
    }
}

impl Supervised {
    fn supervisor(supervisor: Supervisor) -> Self {
        Supervised::Supervisor(supervisor)
    }

    fn children(children: Children) -> Self {
        Supervised::Children(children)
    }

    fn stack(&self) -> ProcStack {
        trace!("Supervised({}): Creating ProcStack.", self.id());
        // FIXME: with_id
        ProcStack::default()
    }

    fn reset(self, bcast: Broadcast) -> RecoverableHandle<Self> {
        debug!(
            "Supervised({}): Resetting to Supervised({}).",
            self.id(),
            bcast.id()
        );
        let stack = self.stack();
        match self {
            Supervised::Supervisor(mut supervisor) => pool::spawn(
                async {
                    supervisor.reset(Some(bcast)).await;
                    Supervised::Supervisor(supervisor)
                },
                stack,
            ),
            Supervised::Children(mut children) => pool::spawn(
                async {
                    children.reset(bcast).await;
                    Supervised::Children(children)
                },
                stack,
            ),
        }
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

    pub(crate) fn elem(&self) -> &BastionPathElement {
        match self {
            // FIXME
            Supervised::Supervisor(supervisor) => {
                supervisor.bcast().path().elem().as_ref().unwrap()
            }
            Supervised::Children(children) => children.bcast().path().elem().as_ref().unwrap(),
        }
    }

    fn callbacks(&self) -> &Callbacks {
        match self {
            Supervised::Supervisor(supervisor) => supervisor.callbacks(),
            Supervised::Children(children) => children.callbacks(),
        }
    }

    fn launch(self) -> RecoverableHandle<Self> {
        debug!("Supervised({}): Launching.", self.id());
        let stack = self.stack();
        match self {
            Supervised::Supervisor(supervisor) => {
                pool::spawn(
                    async {
                        // FIXME: panics?
                        let supervisor = supervisor.launch().await.unwrap();
                        Supervised::Supervisor(supervisor)
                    },
                    stack,
                )
            }
            Supervised::Children(children) => {
                pool::spawn(
                    async {
                        // FIXME: panics?
                        let children = children.launch().await.unwrap();
                        Supervised::Children(children)
                    },
                    stack,
                )
            }
        }
    }
}

impl Default for SupervisionStrategy {
    fn default() -> Self {
        SupervisionStrategy::OneForOne
    }
}

impl Default for ActorRestartStrategy {
    fn default() -> Self {
        ActorRestartStrategy::Immediate
    }
}

impl PartialEq for SupervisorRef {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for SupervisorRef {}
