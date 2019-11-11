//!
//! Supervisors enable users to supervise a subtree of children
//! or other supervisor trees under themselves.
use crate::broadcast::{Broadcast, Parent, Sender};
use crate::children::{Children, ChildrenRef};
use crate::context::BastionId;
use crate::message::{BastionMessage, Deployment, Message};
use bastion_executor::pool;
use futures::prelude::*;
use futures::stream::FuturesOrdered;
use futures::{pending, poll};
use fxhash::FxHashMap;
use lightproc::prelude::*;
use std::iter::FromIterator;
use std::ops::RangeFrom;
use std::task::Poll;

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
/// ```
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
    launched: FxHashMap<BastionId, (usize, RecoverableHandle<Supervised>)>,
    // Supervised children and supervisors that are stopped.
    // This is used when resetting or recovering when the
    // supervision strategy is not "one-for-one".
    stopped: FxHashMap<BastionId, Supervised>,
    strategy: SupervisionStrategy,
    // Whether this supervisor was started by the system (in
    // which case, users shouldn't be able to get a reference
    // to it).
    is_system_supervisor: bool,
    // Messages that were received before the supervisor was
    // started. Those will be "replayed" once a start message
    // is received.
    pre_start_msgs: Vec<BastionMessage>,
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

impl Supervisor {
    pub(crate) fn new(bcast: Broadcast) -> Self {
        let order = Vec::new();
        let launched = FxHashMap::default();
        let stopped = FxHashMap::default();
        let strategy = SupervisionStrategy::default();
        let is_system_supervisor = false;
        let pre_start_msgs = Vec::new();
        let started = false;

        Supervisor {
            bcast,
            order,
            launched,
            stopped,
            strategy,
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
        // FIXME: with_pid
        ProcStack::default()
    }

    /// Resets the supervisor, killing all its supervised elements,
    /// changing its assigned [`Broadcast`] (it is isn't the "system
    /// supervisor") and resetting all its supervised elements.
    ///
    /// [`Broadcast`]: broadcast/struct.Broadcast.html
    pub(crate) async fn reset(&mut self, bcast: Option<Broadcast>) {
        // TODO: stop or kill?
        let killed = self.kill(0..).await;

        // If this is the "system supervisor", it shouldn't change
        // its `Broadcast` because it would make `SYSTEM_SPV`
        // invalid.
        if let Some(bcast) = bcast {
            self.bcast = bcast;
        } else {
            self.bcast.clear_children();
        }

        self.pre_start_msgs.clear();
        self.pre_start_msgs.shrink_to_fit();

        // Reset all the supervised elements with a new `Broadcast`
        // having this supervisor's new `Broadcast` as a parent...
        let mut reset = FuturesOrdered::new();
        for supervised in killed {
            let parent = Parent::supervisor(self.as_ref());
            let bcast = Broadcast::new(parent);

            reset.push(async {
                // FIXME: panics?
                supervised.reset(bcast).await.unwrap()
            })
        }

        // Re-launch all the supervised elements...
        while let Some(supervised) = reset.next().await {
            let id = supervised.id().clone();

            let launched = supervised.launch();
            self.launched
                .insert(id.clone(), (self.order.len(), launched));
            self.order.push(id);
        }

        // If the supervisor was already started before being
        // reset, send a message to all the supervised elements
        // to start them...
        if self.started {
            let msg = BastionMessage::start();
            self.bcast.send_children(msg);
        }
    }

    pub(crate) fn id(&self) -> &BastionId {
        &self.bcast.id()
    }

    pub(crate) fn bcast(&self) -> &Broadcast {
        &self.bcast
    }

    pub(crate) fn as_ref(&self) -> SupervisorRef {
        // TODO: clone or ref?
        let id = self.bcast.id().clone();
        let sender = self.bcast.sender().clone();

        SupervisorRef::new(id, sender)
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
    /// ```
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
        let parent = Parent::supervisor(self.as_ref());
        let bcast = Broadcast::new(parent);

        let supervisor = Supervisor::new(bcast);
        let supervisor = init(supervisor);

        // Send the initialized supervisor over the `Broadcast` so
        // that it will get deployed after this supervisor has been
        // started...
        let msg = BastionMessage::deploy_supervisor(supervisor);
        self.bcast.send_self(msg);

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
    /// ```
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
        let parent = Parent::supervisor(self.as_ref());
        let bcast = Broadcast::new(parent);

        let supervisor = Supervisor::new(bcast);
        let supervisor = init(supervisor);
        let supervisor_ref = supervisor.as_ref();

        // Send the initialized supervisor over the `Broadcast` so
        // that it will get deployed after this supervisor has been
        // started...
        let msg = BastionMessage::deploy_supervisor(supervisor);
        self.bcast.send_self(msg);

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
    /// ```
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
    ///             let opt_msg: Option<Msg> = ctx.try_recv().await;
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
        let parent = Parent::supervisor(self.as_ref());
        let bcast = Broadcast::new(parent);

        let children = Children::new(bcast);
        let mut children = init(children);
        // FIXME: children group elems launched without the group itself being launched
        children.launch_elems();

        // Send the initialized children group over the `Broadcast` so
        // that it will get deployed after this supervisor has been
        // started...
        let msg = BastionMessage::deploy_children(children);
        self.bcast.send_self(msg);

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
    /// ```
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
    ///             let opt_msg: Option<Msg> = ctx.try_recv().await;
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
        let parent = Parent::supervisor(self.as_ref());
        let bcast = Broadcast::new(parent);

        let children = Children::new(bcast);
        let mut children = init(children);
        // FIXME: children group elems launched without the group itself being launched
        children.launch_elems();
        let children_ref = children.as_ref();

        // Send the initialized children group over the `Broadcast` so
        // that it will get deployed after this supervisor has been
        // started...
        let msg = BastionMessage::deploy_children(children);
        self.bcast.send_self(msg);

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
    /// ```
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

    fn stopped(&mut self) {
        self.bcast.stopped();
    }

    fn faulted(&mut self) {
        self.bcast.faulted();
    }

    async fn collect(&mut self, range: RangeFrom<usize>) -> Vec<Supervised> {
        let mut supervised = Vec::new();
        // FIXME: panics?
        for id in self.order.get(range).unwrap() {
            // TODO: Err if None?
            if let Some((_, launched)) = self.launched.remove(&id) {
                // TODO: add a "stopped" list and poll from it instead of awaiting
                supervised.push(launched);
            }
        }

        let supervised = FuturesOrdered::from_iter(supervised.into_iter().rev());
        let mut supervised = supervised.collect::<Vec<_>>().await;

        let mut collected = Vec::with_capacity(supervised.len());
        for id in self.order.drain(..) {
            if let Some(supervised) = self.stopped.remove(&id) {
                collected.push(supervised);

                continue;
            }

            match supervised.pop() {
                Some(Some(supervised)) if supervised.id() == &id => {
                    collected.push(supervised);
                }
                // FIXME
                Some(Some(_)) => unimplemented!(),
                // FIXME
                Some(None) => unimplemented!(),
                // FIXME
                None => unimplemented!(),
            }
        }

        collected
    }

    async fn recover(&mut self, id: BastionId) -> Result<(), ()> {
        match self.strategy {
            SupervisionStrategy::OneForOne => {
                let (order, launched) = self.launched.remove(&id).ok_or(())?;
                // TODO: add a "waiting" list and poll from it instead of awaiting
                // FIXME: panics?
                let supervised = launched.await.unwrap();

                self.bcast.unregister(supervised.id());

                let parent = Parent::supervisor(self.as_ref());
                let bcast = Broadcast::new(parent);
                let id = bcast.id().clone();
                // FIXME: panics?
                let supervised = supervised.reset(bcast).await.unwrap();

                self.bcast.register(supervised.bcast());
                if self.started {
                    let msg = BastionMessage::start();
                    self.bcast.send_child(&id, msg);
                }

                let launched = supervised.launch();
                self.launched.insert(id.clone(), (order, launched));
                self.order[order] = id;
            }
            SupervisionStrategy::OneForAll => {
                // TODO: stop or kill?
                for supervised in self.kill(0..).await {
                    self.bcast.unregister(supervised.id());

                    let parent = Parent::supervisor(self.as_ref());
                    let bcast = Broadcast::new(parent);
                    let id = bcast.id().clone();
                    // FIXME: panics.
                    let supervised = supervised.reset(bcast).await.unwrap();

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
                    // FIXME: panics?
                    let supervised = supervised.reset(bcast).await.unwrap();

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
                self.stopped();

                return Err(());
            }
            BastionMessage::Kill => {
                self.kill(0..).await;
                self.stopped();

                return Err(());
            }
            BastionMessage::Deploy(deployment) => match deployment {
                Deployment::Supervisor(supervisor) => {
                    self.bcast.register(&supervisor.bcast);
                    if self.started {
                        let msg = BastionMessage::start();
                        self.bcast.send_child(supervisor.id(), msg);
                    }

                    let id = supervisor.id().clone();
                    let supervised = Supervised::supervisor(supervisor);

                    let launched = supervised.launch();
                    self.launched
                        .insert(id.clone(), (self.order.len(), launched));
                    self.order.push(id);
                }
                Deployment::Children(children) => {
                    self.bcast.register(children.bcast());
                    if self.started {
                        let msg = BastionMessage::start();
                        self.bcast.send_child(children.id(), msg);
                    }

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
            BastionMessage::Stopped { id } => {
                // FIXME: Err if None?
                if let Some((_, launched)) = self.launched.remove(&id) {
                    // TODO: add a "waiting" list an poll from it instead of awaiting
                    // FIXME: panics?
                    let supervised = launched.await.unwrap();

                    self.bcast.unregister(&id);
                    self.stopped.insert(id, supervised);
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

    async fn run(mut self) -> Self {
        loop {
            match poll!(&mut self.bcast.next()) {
                // TODO: Err if started == true?
                Poll::Ready(Some(BastionMessage::Start)) => {
                    self.started = true;

                    let msg = BastionMessage::start();
                    self.bcast.send_children(msg);

                    let msgs = self.pre_start_msgs.drain(..).collect::<Vec<_>>();
                    self.pre_start_msgs.shrink_to_fit();

                    for msg in msgs {
                        if self.handle(msg).await.is_err() {
                            return self;
                        }
                    }
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

    pub(crate) fn launch(self) -> RecoverableHandle<Self> {
        let stack = self.stack();
        pool::spawn(self.run(), stack)
    }
}

impl SupervisorRef {
    pub(crate) fn new(id: BastionId, sender: Sender) -> Self {
        SupervisorRef { id, sender }
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
        let parent = Parent::supervisor(self.clone());
        let bcast = Broadcast::new(parent);

        let supervisor = Supervisor::new(bcast);
        let supervisor = init(supervisor);
        let supervisor_ref = supervisor.as_ref();

        let msg = BastionMessage::deploy_supervisor(supervisor);
        self.send(msg).map_err(|_| ())?;

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
    ///             let opt_msg: Option<Msg> = ctx.try_recv().await;
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
        let parent = Parent::supervisor(self.clone());
        let bcast = Broadcast::new(parent);

        let children = Children::new(bcast);
        let mut children = init(children);

        // FIXME: children group elems launched without the group itself being launched
        children.launch_elems();
        let children_ref = children.as_ref();

        let msg = BastionMessage::deploy_children(children);
        self.send(msg).map_err(|_| ())?;

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
        let msg = BastionMessage::supervise_with(strategy);
        self.send(msg).map_err(|_| ())
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
        let msg = BastionMessage::broadcast(msg);
        // FIXME: panics?
        self.send(msg).map_err(|msg| msg.into_msg().unwrap())
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
        let msg = BastionMessage::stop();
        self.send(msg).map_err(|_| ())
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
        let msg = BastionMessage::kill();
        self.send(msg).map_err(|_| ())
    }

    pub(crate) fn send(&self, msg: BastionMessage) -> Result<(), BastionMessage> {
        self.sender
            .unbounded_send(msg)
            .map_err(|err| err.into_inner())
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

    fn reset(self, bcast: Broadcast) -> RecoverableHandle<Self> {
        match self {
            Supervised::Supervisor(mut supervisor) => {
                // FIXME: with_pid
                let stack = ProcStack::default();
                pool::spawn(
                    async {
                        supervisor.reset(Some(bcast)).await;
                        Supervised::Supervisor(supervisor)
                    },
                    stack,
                )
            }
            Supervised::Children(mut children) => {
                // FIXME: with_pid
                let stack = ProcStack::default();
                pool::spawn(
                    async {
                        children.reset(bcast).await;
                        Supervised::Children(children)
                    },
                    stack,
                )
            }
        }
    }

    fn launch(self) -> RecoverableHandle<Self> {
        match self {
            Supervised::Supervisor(supervisor) => {
                // FIXME: with_pid
                let stack = ProcStack::default();
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
                // FIXME: with_pid
                let stack = ProcStack::default();
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
