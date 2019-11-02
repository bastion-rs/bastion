use crate::broadcast::{Broadcast, Parent};
use crate::children::ChildrenRef;
use crate::context::BastionContext;
use crate::message::{BastionMessage, Message};
use crate::supervisor::{Supervisor, SupervisorRef};
use crate::system::{ROOT_SPV, SYSTEM, SYSTEM_SENDER};
use std::future::Future;
use std::thread;

pub struct Bastion {
    _priv: (),
}

impl Bastion {
    /// Initializes the system if it hasn't already been done.
    ///
    /// **It is required that you call this method at least once
    /// before using any of bastion's features.**
    ///
    /// # Example
    ///
    /// ```
    /// use bastion::prelude::*;
    ///
    /// fn main() {
    ///     Bastion::init();
    ///
    ///     // You can now use bastion...
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// }
    /// ```
    pub fn init() {
        std::panic::set_hook(Box::new(|_| ()));

        // NOTE: this is just to make sure that SYSTEM_SENDER has been initialized by lazy_static
        SYSTEM_SENDER.is_closed();
    }

    /// Creates a new supervisor, passes it through the specified
    /// `init` closure and then sends it to the system for it to
    /// start supervising children.
    ///
    /// This method returns a [`SupervisorRef`] for the newly
    /// created supervisor if it succeeded, or `Err(())`
    /// otherwise.
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
    /// let sp_ref: SupervisorRef = Bastion::supervisor(|sp| {
    ///     // Configure the supervisor...
    ///     sp.strategy(SupervisionStrategy::OneForOne)
    ///     // ...and return it.
    /// }).expect("Couldn't create the supervisor.");
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    ///
    /// [`SupervisorRef`]: supervisor/struct.SupervisorRef.html
    pub fn supervisor<S>(init: S) -> Result<SupervisorRef, ()>
    where
        S: FnOnce(Supervisor) -> Supervisor,
    {
        let parent = Parent::system();
        let bcast = Broadcast::new(parent);

        let supervisor = Supervisor::new(bcast);
        let supervisor = init(supervisor);
        let supervisor_ref = supervisor.as_ref();

        let msg = BastionMessage::deploy_supervisor(supervisor);
        SYSTEM_SENDER.unbounded_send(msg).map_err(|_| ())?;

        Ok(supervisor_ref)
    }

    /// Creates a new group of children that will run the future
    /// returned by `init` and then makes the system's default
    /// supervisor supervise it. The group will have as many
    /// elements as defined by `redundancy` and if one of them
    /// stops or dies, all of the other elements of the group
    /// will be stopped or killed.
    ///
    /// The future of each element will need to return a `Result<(), ()>`,
    /// where `Ok(())` indicates that the element has stopped and
    /// `Err(())` that it died, in which case it will be restarted by the
    /// default supervisor.
    ///
    /// This method returns a [`ChildrenRef`] for the newly
    /// created children group if it succeeded, or `Err(())`
    /// otherwise.
    ///
    /// # Arguments
    ///
    /// * `init` - A closure taking a [`BastionContext`] as an
    ///     argument and returning the [`Future`] that every
    ///     element of the children group will run.
    /// * `redundancy` - How many elements the children group
    ///     should contain. Each element of the group will be
    ///     independent, capable of sending and receiving its
    ///     own messages but will be stopped or killed if
    ///     another element stopped or died.
    ///
    /// # Example
    ///
    /// ```
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    /// let children_ref: ChildrenRef = Bastion::children(|ctx: BastionContext|
    ///     async move {
    ///         // Send and receive messages...
    ///         let opt_msg: Option<Msg> = ctx.try_recv().await;
    ///         // ...and return `Ok(())` or `Err(())` when you are done...
    ///         Ok(())
    ///
    ///         // Note that if `Err(())` was returned, the supervisor would
    ///         // restart the children group.
    ///     },
    ///     1
    /// ).expect("Couldn't create the children group.");
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    ///
    /// [`ChildrenRef`]: children/struct.ChildrenRef.html
    /// [`BastionContext`]: struct.BastionContext.html
    /// [`Future`]: https://doc.rust-lang.org/std/future/trait.Future.html
    pub fn children<C, F>(init: C, redundancy: usize) -> Result<ChildrenRef, ()>
    where
        C: Fn(BastionContext) -> F + Send + Sync + 'static,
        F: Future<Output = Result<(), ()>> + Send + 'static,
    {
        // FIXME: panics
        ROOT_SPV
            .clone()
            .read()
            .wait()
            .unwrap()
            .as_ref()
            .unwrap()
            .children(init, redundancy)
    }

    /// Sends a message to the system which will then send it to all
    /// the root-level supervisors and their supervised children and
    /// supervisors, etc.
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
    /// let msg = "A message containing data.";
    /// Bastion::broadcast(msg).expect("Couldn't send the message.");
    ///
    ///     # Bastion::children(|ctx: BastionContext|
    ///         # async move {
    /// // And then in every children groups's elements' future...
    /// msg! { ctx.recv().await?,
    ///     ref msg: &'static str => {
    ///         assert_eq!(msg, &"A message containing data.");
    ///     };
    ///     // We are only broadcasting a `&'static str` in this
    ///     // example, so we know that this won't happen...
    ///     _: _ => ();
    /// }
    ///             #
    ///             # Ok(())
    ///         # },
    ///         # 1,
    ///     # ).unwrap();
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    pub fn broadcast<M: Message>(msg: M) -> Result<(), M> {
        let msg = BastionMessage::broadcast(msg);
        // FIXME: panics?
        SYSTEM_SENDER
            .unbounded_send(msg)
            .map_err(|err| err.into_inner().into_msg().unwrap())
    }

    /// Sends a message to the system to tell it to start
    /// handling messages and running children.
    ///
    /// # Example
    ///
    /// ```
    /// use bastion::prelude::*;
    ///
    /// fn main() {
    ///     Bastion::init();
    ///
    ///     // Use bastion, spawn children and supervisors...
    ///
    ///     Bastion::start();
    ///
    ///     // The system will soon start, messages will
    ///     // now be handled...
    ///     #
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// }
    /// ```
    pub fn start() {
        let msg = BastionMessage::start();
        // FIXME: Err(Error)
        SYSTEM_SENDER.unbounded_send(msg).ok();
    }

    /// Sends a message to the system to tell it to stop
    /// every running children groups and supervisors.
    ///
    /// # Example
    ///
    /// ```
    /// use bastion::prelude::*;
    ///
    /// fn main() {
    ///     Bastion::init();
    ///
    ///     // Use bastion, spawn children and supervisors...
    ///
    ///     Bastion::start();
    ///
    ///     // Send messages to children and/or do some
    ///     // work until you decide to stop the system...
    ///
    ///     Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// }
    /// ```
    pub fn stop() {
        let msg = BastionMessage::stop();
        // FIXME: Err(Error)
        SYSTEM_SENDER.unbounded_send(msg).ok();
    }

    /// Sends a message to the system to tell it to kill every
    /// running children groups and supervisors
    ///
    /// # Example
    ///
    /// ```
    /// use bastion::prelude::*;
    ///
    /// fn main() {
    ///     Bastion::init();
    ///
    ///     // Use bastion, spawn children and supervisors...
    ///
    ///     Bastion::start();
    ///     // Send messages to children and/or do some
    ///     // work until you decide to kill the system...
    ///
    ///     Bastion::kill();
    ///     # Bastion::block_until_stopped();
    /// }
    /// ```
    pub fn kill() {
        let msg = BastionMessage::kill();
        // FIXME: Err(Error)
        SYSTEM_SENDER.unbounded_send(msg).ok();

        // FIXME: panics
        let mut system = SYSTEM.clone().lock().wait().unwrap();
        if let Some(system) = system.take() {
            system.cancel();
        }
    }

    /// Blocks the current thread until the system is stopped
    /// (either by calling [`Bastion::stop()`] or
    /// [`Bastion::kill`]).
    ///
    /// # Example
    ///
    /// ```
    /// use bastion::prelude::*;
    ///
    /// fn main() {
    ///     Bastion::init();
    ///
    ///     // Use bastion, spawn children and supervisors...
    ///
    ///     Bastion::start();
    ///     // Send messages to children and/or do some
    ///     // work...
    ///
    ///     # Bastion::stop();
    ///     Bastion::block_until_stopped();
    ///     // The system is now stopped. A child might have
    ///     // stopped or killed it...
    /// }
    /// ```
    ///
    /// [`Bastion::stop()`]: #method.stop
    /// [`Bastion::kill()`]: #method.kill
    pub fn block_until_stopped() {
        loop {
            // FIXME: panics
            let system = SYSTEM.clone().lock().wait().unwrap();
            if system.is_none() {
                return;
            }

            thread::yield_now();
        }
    }
}
