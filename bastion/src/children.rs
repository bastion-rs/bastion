//!
//! Children are a group of child supervised under a supervisor
use crate::broadcast::{Broadcast, Parent, Sender};
use crate::callbacks::Callbacks;
use crate::child::{Child, Init};
use crate::child_ref::ChildRef;
use crate::children_ref::ChildrenRef;
use crate::context::{BastionContext, BastionId, ContextState};
use crate::dispatcher::Dispatcher;
use crate::envelope::Envelope;
use crate::message::BastionMessage;
use crate::path::BastionPathElement;
use crate::system::SYSTEM;
use bastion_executor::pool;
use futures::pending;
use futures::poll;
use futures::prelude::*;
use futures::stream::{FuturesOrdered, FuturesUnordered};
use fxhash::FxHashMap;
use lightproc::prelude::*;
use qutex::Qutex;
use std::fmt::Debug;
use std::future::Future;
use std::iter::FromIterator;
use std::sync::Arc;
use std::task::Poll;

#[derive(Debug)]
/// A children group that will contain a defined number of
/// elements (set with [`with_redundancy`] or `1` by default)
/// all running a future (returned by the closure that is set
/// with [`with_exec`]).
///
/// When an element of the group stops or panics, all the
/// elements will be stopped as well and the group's supervisor
/// will receive a notice that a stop or panic occurred (note
/// that if a panic occurred, the supervisor will restart the
/// children group and eventually some of its other children,
/// depending on its [`SupervisionStrategy`]).
///
/// # Example
///
/// ```rust
/// # use bastion::prelude::*;
/// #
/// # fn main() {
///     # Bastion::init();
///     #
/// let children_ref: ChildrenRef = Bastion::children(|children| {
///     // Configure the children group...
///     children.with_exec(|ctx: BastionContext| {
///         async move {
///             // Send and receive messages...
///             let opt_msg: Option<SignedMessage> = ctx.try_recv().await;
///             // ...and return `Ok(())` or `Err(())` when you are done...
///             Ok(())
///
///             // Note that if `Err(())` was returned, the supervisor would
///             // restart the children group.
///         }
///     })
///     // ...and return it.
/// }).expect("Couldn't create the children group.");
///     #
///     # Bastion::start();
///     # Bastion::stop();
///     # Bastion::block_until_stopped();
/// # }
/// ```
///
/// [`with_redundancy`]: #method.with_redundancy
/// [`with_exec`]: #method.with_exec
/// [`SupervisionStrategy`]: supervisor/enum.SupervisionStrategy.html
pub struct Children {
    bcast: Broadcast,
    // The currently launched elements of the group.
    launched: FxHashMap<BastionId, (Sender, RecoverableHandle<()>)>,
    // The closure returning the future that will be used by
    // every element of the group.
    init: Init,
    redundancy: usize,
    // The callbacks called at the group's different lifecycle
    // events.
    callbacks: Callbacks,
    // Messages that were received before the group was
    // started. Those will be "replayed" once a start message
    // is received.
    pre_start_msgs: Vec<Envelope>,
    started: bool,
    // List of dispatchers attached to each actor in the group.
    dispatchers: Vec<Arc<Box<Dispatcher>>>,
}

impl Children {
    pub(crate) fn new(bcast: Broadcast) -> Self {
        debug!("Children({}): Initializing.", bcast.id());
        let launched = FxHashMap::default();
        let init = Init::default();
        let redundancy = 1;
        let callbacks = Callbacks::new();
        let pre_start_msgs = Vec::new();
        let started = false;
        let dispatchers = Vec::new();

        Children {
            bcast,
            launched,
            init,
            redundancy,
            callbacks,
            pre_start_msgs,
            started,
            dispatchers,
        }
    }

    fn stack(&self) -> ProcStack {
        trace!("Children({}): Creating ProcStack.", self.id());
        // FIXME: with_pid
        ProcStack::default()
    }

    pub(crate) async fn reset(&mut self, bcast: Broadcast) {
        debug!(
            "Children({}): Resetting to Children({}).",
            self.id(),
            bcast.id()
        );
        // TODO: stop or kill?
        self.kill().await;

        self.bcast = bcast;
        self.started = false;

        trace!(
            "Children({}): Removing {} pre-start messages.",
            self.id(),
            self.pre_start_msgs.len()
        );
        self.pre_start_msgs.clear();
        self.pre_start_msgs.shrink_to_fit();

        self.launch_elems();
    }

    /// Returns this children group's identifier.
    ///
    /// Note that the children group's identifier is reset when it
    /// is restarted.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    /// Bastion::children(|children| {
    ///     let children_id: &BastionId = children.id();
    ///     // ...
    ///     # children
    /// }).expect("Couldn't create the children group.");
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    pub fn id(&self) -> &BastionId {
        self.bcast.id()
    }

    pub(crate) fn bcast(&self) -> &Broadcast {
        &self.bcast
    }

    pub(crate) fn callbacks(&self) -> &Callbacks {
        &self.callbacks
    }

    pub(crate) fn as_ref(&self) -> ChildrenRef {
        trace!(
            "Children({}): Creating new ChildrenRef({}).",
            self.id(),
            self.id()
        );
        // TODO: clone or ref?
        let id = self.bcast.id().clone();
        let sender = self.bcast.sender().clone();
        let path = self.bcast.path().clone();

        let mut children = Vec::with_capacity(self.launched.len());
        for (id, (sender, _)) in &self.launched {
            trace!("Children({}): Creating new ChildRef({}).", self.id(), id);
            // TODO: clone or ref?
            let child = ChildRef::new(id.clone(), sender.clone(), path.clone());
            children.push(child);
        }

        let dispatchers = self
            .dispatchers
            .iter()
            .map(|dispatcher| dispatcher.dispatcher_type())
            .collect();

        ChildrenRef::new(id, sender, path, children, dispatchers)
    }

    /// Sets the closure taking a [`BastionContext`] and returning a
    /// [`Future`] that will be used by every element of this children
    /// group.
    ///
    /// When a new element is started, it will be assigned a new context,
    /// pass it to the `init` closure and poll the returned future until
    /// it stops, panics or another element of the group stops or panics.
    ///
    /// The returned future's output should be `Result<(), ()>`.
    ///
    /// # Arguments
    ///
    /// * `init` - The closure taking a [`BastionContext`] and returning
    ///     a [`Future`] that will be used by every element of this
    ///     children group.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    /// Bastion::children(|children| {
    ///     children.with_exec(|ctx| {
    ///         async move {
    ///             // Send and receive messages...
    ///             let opt_msg: Option<SignedMessage> = ctx.try_recv().await;
    ///             // ...and return `Ok(())` or `Err(())` when you are done...
    ///             Ok(())
    ///
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
    pub fn with_exec<I, F>(mut self, init: I) -> Self
    where
        I: Fn(BastionContext) -> F + Send + Sync + 'static,
        F: Future<Output = Result<(), ()>> + Send + 'static,
    {
        trace!("Children({}): Setting exec closure.", self.id());
        self.init = Init::new(init);
        self
    }

    /// Sets the number of number of elements this children group will
    /// contain. Each element will call the closure passed in
    /// [`with_exec`] and run the returned future until it stops,
    /// panics or another element in the group stops or panics.
    ///
    /// The default number of elements a children group contains is `1`.
    ///
    /// # Arguments
    ///
    /// * `redundancy` - The number of elements this group will contain.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    /// Bastion::children(|children| {
    ///     // Note that "1" is the default number of elements.
    ///     children.with_redundancy(1)
    /// }).expect("Couldn't create the children group.");
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    ///
    /// [`with_exec`]: #method.with_exec
    pub fn with_redundancy(mut self, redundancy: usize) -> Self {
        trace!(
            "Children({}): Setting redundancy: {}",
            self.id(),
            redundancy
        );
        if redundancy == std::usize::MIN {
            self.redundancy = redundancy.saturating_add(1);
        } else {
            self.redundancy = redundancy;
        }

        self
    }

    /// Appends each supervised element to the declared dispatcher.
    ///
    /// By default supervised elements aren't added to any of dispatcher.
    ///
    /// # Arguments
    ///
    /// * `redundancy` - An instance of struct that implements the
    /// [`DispatcherHandler`] trait.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    /// Bastion::children(|children| {
    ///     children
    ///         .with_dispatcher(
    ///             Dispatcher::default()
    ///                 .with_dispatcher_type(DispatcherType::Named("CustomGroup".to_string()))
    ///         )
    /// }).expect("Couldn't create the children group.");
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    /// [`DispatcherHandler`]: ../dispatcher/trait.DispatcherHandler.html
    pub fn with_dispatcher(mut self, dispatcher: Dispatcher) -> Self {
        self.dispatchers.push(Arc::new(Box::new(dispatcher)));
        self
    }

    /// Sets the callbacks that will get called at this children group's
    /// different lifecycle events.
    ///
    /// See [`Callbacks`]'s documentation for more information about the
    /// different callbacks available.
    ///
    /// # Arguments
    ///
    /// * `callbacks` - The callbacks that will get called for this
    ///     children group.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    /// Bastion::children(|children| {
    ///     let callbacks = Callbacks::new()
    ///         .with_before_start(|| println!("Children group started."))
    ///         .with_after_stop(|| println!("Children group stopped."));
    ///
    ///     children
    ///         .with_callbacks(callbacks)
    ///         .with_exec(|ctx| {
    ///             // -- Children group started.
    ///             async move {
    ///                 // ...
    ///                 # Ok(())
    ///             }
    ///             // -- Children group stopped.
    ///         })
    /// }).expect("Couldn't create the children group.");
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
            "Children({}): Setting callbacks: {:?}",
            self.id(),
            callbacks
        );
        self.callbacks = callbacks;
        self
    }

    async fn stop(&mut self) {
        debug!("Children({}): Stopping.", self.id());
        self.bcast.stop_children();

        let launched = self.launched.drain().map(|(_, (_, launched))| launched);
        FuturesUnordered::from_iter(launched)
            .for_each_concurrent(None, |_| async {
                trace!("Children({}): Unknown child stopped.", self.id());
            })
            .await;
    }

    async fn kill(&mut self) {
        debug!("Children({}): Killing.", self.id());
        self.bcast.kill_children();

        let mut children = FuturesOrdered::new();
        for (_, (_, launched)) in self.launched.drain() {
            launched.cancel();

            children.push(launched);
        }

        children
            .for_each_concurrent(None, |_| async {
                trace!("Children({}): Unknown child stopped.", self.id());
            })
            .await;
    }

    fn stopped(&mut self) {
        debug!("Children({}): Stopped.", self.id());
        self.remove_dispatchers();
        self.bcast.stopped();
    }

    fn faulted(&mut self) {
        debug!("Children({}): Faulted.", self.id());
        self.remove_dispatchers();
        self.bcast.faulted();
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
                self.stop().await;
                self.stopped();

                return Err(());
            }
            Envelope {
                msg: BastionMessage::Kill,
                ..
            } => {
                self.kill().await;
                self.stopped();

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
            // FIXME
            Envelope {
                msg: BastionMessage::SuperviseWith(_),
                ..
            } => unimplemented!(),
            Envelope {
                msg: BastionMessage::Message(ref message),
                ..
            } => {
                debug!(
                    "Children({}): Broadcasting a message: {:?}",
                    self.id(),
                    message
                );
                self.bcast.send_children(env);
            }
            Envelope {
                msg: BastionMessage::Stopped { id },
                ..
            } => {
                // FIXME: Err if false?
                if self.launched.contains_key(&id) {
                    debug!("Children({}): Child({}) stopped.", self.id(), id);
                    self.stop().await;
                    self.stopped();

                    return Err(());
                }
            }
            Envelope {
                msg: BastionMessage::Faulted { id },
                ..
            } => {
                // FIXME: Err if false?
                if self.launched.contains_key(&id) {
                    warn!("Children({}): Child({}) faulted.", self.id(), id);
                    self.kill().await;
                    self.faulted();

                    return Err(());
                }
            }
        }

        Ok(())
    }

    async fn run(mut self) -> Self {
        debug!("Children({}): Launched.", self.id());
        self.register_dispatchers();

        loop {
            for (_, launched) in self.launched.values_mut() {
                let _ = poll!(launched);
            }

            match poll!(&mut self.bcast.next()) {
                // TODO: Err if started == true?
                Poll::Ready(Some(Envelope {
                    msg: BastionMessage::Start,
                    ..
                })) => {
                    trace!(
                        "Children({}): Received a new message (started=false): {:?}",
                        self.id(),
                        BastionMessage::Start
                    );
                    debug!("Children({}): Starting.", self.id());
                    self.started = true;

                    let msg = BastionMessage::start();
                    let env =
                        Envelope::new(msg, self.bcast.path().clone(), self.bcast.sender().clone());
                    self.bcast.send_children(env);

                    let msgs = self.pre_start_msgs.drain(..).collect::<Vec<_>>();
                    self.pre_start_msgs.shrink_to_fit();

                    debug!(
                        "Children({}): Replaying messages received before starting.",
                        self.id()
                    );
                    for msg in msgs {
                        trace!("Children({}): Replaying message: {:?}", self.id(), msg);
                        if self.handle(msg).await.is_err() {
                            return self;
                        }
                    }
                }
                Poll::Ready(Some(msg)) if !self.started => {
                    trace!(
                        "Children({}): Received a new message (started=false): {:?}",
                        self.id(),
                        msg
                    );
                    self.pre_start_msgs.push(msg);
                }
                Poll::Ready(Some(msg)) => {
                    trace!(
                        "Children({}): Received a new message (started=true): {:?}",
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

    pub(crate) fn launch_elems(&mut self) {
        debug!("Children({}): Launching elements.", self.id());
        for _ in 0..self.redundancy {
            let parent = Parent::children(self.as_ref());
            let bcast = Broadcast::new(parent, BastionPathElement::Child(BastionId::new()));

            // TODO: clone or ref?
            let id = bcast.id().clone();
            let sender = bcast.sender().clone();
            let path = bcast.path().clone();
            let child_ref = ChildRef::new(id.clone(), sender.clone(), path);

            let children = self.as_ref();
            let supervisor = self.bcast.parent().clone().into_supervisor();

            let state = ContextState::new();
            let state = Qutex::new(state);

            let ctx = BastionContext::new(id, child_ref.clone(), children, supervisor, state.clone());
            let exec = (self.init.0)(ctx);

            self.bcast.register(&bcast);

            debug!(
                "Children({}): Initializing Child({}).",
                self.id(),
                bcast.id()
            );
            let child = Child::new(exec, bcast, state, child_ref);
            debug!("Children({}): Launching Child({}).", self.id(), child.id());
            let id = child.id().clone();
            let launched = child.launch();

            self.launched.insert(id, (sender, launched));
        }
    }

    pub(crate) fn launch(self) -> RecoverableHandle<Self> {
        debug!("Children({}): Launching.", self.id());
        let stack = self.stack();
        pool::spawn(self.run(), stack)
    }

    /// Registers all declared local dispatchers in the global dispatcher.
    pub(crate) fn register_dispatchers(&self) {
        let global_dispatcher = SYSTEM.dispatcher();

        for dispatcher in self.dispatchers.iter() {
            global_dispatcher.register_dispatcher(dispatcher)
        }
    }

    /// Removes all declared local dispatchers from the global dispatcher.
    pub(crate) fn remove_dispatchers(&self) {
        let global_dispatcher = SYSTEM.dispatcher();

        for dispatcher in self.dispatchers.iter() {
            global_dispatcher.remove_dispatcher(dispatcher)
        }
    }
}
