use crate::children::{ChildRef, ChildrenRef};
use crate::message::Msg;
use crate::supervisor::SupervisorRef;
use futures::pending;
use qutex::{Guard, Qutex};
use std::collections::VecDeque;
use uuid::Uuid;

pub(crate) const NIL_ID: BastionId = BastionId(Uuid::nil());

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub(crate) struct BastionId(Uuid);

#[derive(Debug)]
pub struct BastionContext {
    id: BastionId,
    child: ChildRef,
    children: ChildrenRef,
    supervisor: Option<SupervisorRef>,
    state: Qutex<ContextState>,
}

#[derive(Debug)]
pub(crate) struct ContextState {
    msgs: VecDeque<Msg>,
}

impl BastionId {
    pub(crate) fn new() -> Self {
        let uuid = Uuid::new_v4();

        BastionId(uuid)
    }
}

impl BastionContext {
    pub(crate) fn new(
        id: BastionId,
        child: ChildRef,
        children: ChildrenRef,
        supervisor: Option<SupervisorRef>,
        state: Qutex<ContextState>,
    ) -> Self {
        BastionContext {
            id,
            child,
            children,
            supervisor,
            state,
        }
    }

    /// Returns a [`ChildRef`] referencing the children group's
    /// element that is linked to this `BastionContext`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    /// Bastion::children(|children| {
    ///     children.with_exec(|ctx: BastionContext| {
    ///         async move {
    ///             let current: &ChildRef = ctx.current();
    ///             // Stop or kill the current element (note that this will
    ///             // only take effect after this future becomes "pending")...
    ///
    ///             Ok(())
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
    /// [`ChildRef`]: children/struct.ChildRef.html
    pub fn current(&self) -> &ChildRef {
        &self.child
    }

    /// Returns a [`ChildrenRef`] referencing the children group
    /// of the element that is linked to this `BastionContext`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    /// Bastion::children(|children| {
    ///     children.with_exec(|ctx: BastionContext| {
    ///         async move {
    ///             let parent: &ChildrenRef = ctx.parent();
    ///             // Get the other elements of the group, broadcast message,
    ///             // or stop or kill the children group...
    ///
    ///             Ok(())
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
    /// [`ChildrenRef`]: children/struct.ChildrenRef.html
    pub fn parent(&self) -> &ChildrenRef {
        &self.children
    }

    /// Returns a [`SupervisorRef`] referencing the supervisor
    /// that supervises the element that is linked to this
    /// `BastionContext` if it isn't the system's supervisor
    /// (if the children group wasn't created using
    /// [`Bastion::children`]).
    ///
    /// # Example
    ///
    /// ```
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    /// // When calling the method from a children group supervised
    /// // by a supervisor created by the user...
    /// Bastion::supervisor(|sp| {
    ///     sp.children(|children| {
    ///         children.with_exec(|ctx: BastionContext| {
    ///             async move {
    ///                 // ...the method will return a SupervisorRef referencing the
    ///                 // user-created supervisor...
    ///                 let supervisor: Option<&SupervisorRef> = ctx.supervisor(); // Some
    ///                 assert!(supervisor.is_some());
    ///
    ///                 Ok(())
    ///             }
    ///         })
    ///     })
    /// }).expect("Couldn't create the supervisor.");
    ///
    /// // When calling the method from a children group supervised
    /// // by the system's supervisor...
    /// Bastion::children(|children| {
    ///     children.with_exec(|ctx: BastionContext| {
    ///         async move {
    ///             // ...the method won't return a SupervisorRef...
    ///             let supervisor: Option<&SupervisorRef> = ctx.supervisor(); // None
    ///             assert!(supervisor.is_none());
    ///
    ///             Ok(())
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
    /// [`SupervisorRef`]: supervisor/struct.SupervisorRef.html
    /// [`Bastion::children`]: struct.Bastion.html#method.children
    pub fn supervisor(&self) -> Option<&SupervisorRef> {
        self.supervisor.as_ref()
    }

    /// Tries to retrieve asynchronously a message received by
    /// the element this `BastionContext` is linked to.
    ///
    /// If you need to wait (always asynchronously) until at
    /// least one message can be retrieved, use [`recv`] instead.
    ///
    /// This method returns [`Msg`] if a message was available, or
    /// `None otherwise.
    ///
    /// # Example
    ///
    /// ```
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    /// Bastion::children(|children| {
    ///     children.with_exec(|ctx: BastionContext| {
    ///         async move {
    ///             let opt_msg: Option<Msg> = ctx.try_recv().await;
    ///             // If a message was received by the element, `opt_msg` will
    ///             // be `Some(Msg)`, otherwise it will be `None`.
    ///
    ///             Ok(())
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
    /// [`recv`]: #method.recv
    /// [`Msg`]: children/struct.Msg.html
    pub async fn try_recv(&self) -> Option<Msg> {
        // TODO: Err(Error)
        let mut state = self.state.clone().lock_async().await.ok()?;

        state.msgs.pop_front()
    }

    /// Retrieves asynchronously a message received by the element
    /// this `BastionContext` is linked to and waits (always
    /// asynchronously) for one if none has been received yet.
    ///
    /// If you don't need to wait until at least one message
    /// can be retrieved, use [`try_recv`] instead.
    ///
    /// This method returns [`Msg`] if it succeeded, or `Err(())`
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
    /// Bastion::children(|children| {
    ///     children.with_exec(|ctx: BastionContext| {
    ///         async move {
    ///             // This will block until a message has been received...
    ///             let msg: Msg = ctx.recv().await?;
    ///
    ///             Ok(())
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
    /// [`try_recv`]: #method.try_recv
    /// [`Msg`]: children/struct.Msg.html
    pub async fn recv(&self) -> Result<Msg, ()> {
        loop {
            // TODO: Err(Error)
            let mut state = self.state.clone().lock_async().await.unwrap();

            if let Some(msg) = state.msgs.pop_front() {
                return Ok(msg);
            }

            Guard::unlock(state);

            pending!();
        }
    }
}

impl ContextState {
    pub(crate) fn new() -> Self {
        let msgs = VecDeque::new();

        ContextState { msgs }
    }

    pub(crate) fn push_msg(&mut self, msg: Msg) {
        self.msgs.push_back(msg)
    }
}
