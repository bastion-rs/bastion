use crate::children::ChildrenRef;
use crate::context::BastionId;
use crate::message::BastionMessage;
use crate::supervisor::SupervisorRef;
use crate::system::SYSTEM;
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::prelude::*;
use fxhash::FxHashMap;
use std::pin::Pin;
use std::task::{Context, Poll};

pub(crate) type Sender = UnboundedSender<BastionMessage>;
pub(crate) type Receiver = UnboundedReceiver<BastionMessage>;

#[derive(Debug)]
pub(crate) struct Broadcast {
    id: BastionId,
    sender: Sender,
    recver: Receiver,
    parent: Parent,
    children: FxHashMap<BastionId, Sender>,
}

#[derive(Debug, Clone)]
pub(crate) enum Parent {
    /// Used by the [`System`] to indicate that it doesn't have a
    /// parent.
    ///
    /// [`System`]: /system/struct.System.html
    None,
    /// Used by the [`Supervisor`]s created via [`Bastion::supervisor`]
    /// and, thus, directly supervised by the [`System`].
    ///
    /// [`Supervisor`]: /supervisor/struct.Supervisor.html
    /// [`Bastion::supervisor`]: /struct.Bastion.html#method.supervisor
    /// [`System`]: /system/struct.System.html
    System,
    /// Used by the [`Supervisor`]s created via [`Supervisor::supervisor`],
    /// [`Supervisor::supervisor_ref`] and [`SupervisorRef::supervisor`] and
    /// all [`Children`]s.
    ///
    /// This contains a [`SupervisorRef`] referencing the supervisor that
    /// the supervised element was started on.
    ///
    /// [`Supervisor`]: /supervisor/struct.Supervisor.html
    /// [`Supervisor::supervisor`]: /supervisor/struct.Supervisor.html#method.supervisor
    /// [`Supervisor::supervisor_ref`]: /supervisor/struct.Supervisor.html#method.supervisor
    /// [`SupervisorRef::supervisor`]: /supervisor/struct.SupervisorRef.html#method.supervisor
    /// [`Children`]: /children/struct.Children.html
    /// [`SupervisorRef`]: /supervisor/struct.SupervisorRef.html
    Supervisor(SupervisorRef),
    /// Used by the [`Child`]s.
    ///
    /// This contains a [`ChildrenRef`] referencing the children group that
    /// the `Child` was started on.
    ///
    /// [`Child`]: /children/struct.Child.html
    /// [`ChildrenRef`]: /children/struct.ChildrenRef.html
    Children(ChildrenRef),
}

impl Broadcast {
    /// Creates a new instance `Broadcast` with the specified
    /// [`Parent`], making it generate a [`BastionId`] for itself.
    ///
    /// This should get called when creating a new [`Supervisor`],
    /// [`Children`] or [`Child`] or resetting one of these.
    ///
    /// # Arguments
    ///
    /// * `parent` - The parent of this `Broadcast`.
    ///
    /// [`Parent`]: /broadcast/enum.Parent.html
    /// [`BastionId`]: /context/struct.BastionId.html
    /// [`Supervisor`]: /supervisor/struct.Supervisor.html
    /// [`Children`]: /children/struct.Children.html
    /// [`Child`]: /children/struct.Child.html
    pub(crate) fn new(parent: Parent) -> Self {
        let id = BastionId::new();
        let (sender, recver) = mpsc::unbounded();
        let children = FxHashMap::default();

        Broadcast {
            id,
            parent,
            sender,
            recver,
            children,
        }
    }

    /// Creates a new instance `Broadcast` with the specified
    /// [`Parent`] and [`BastionId`].
    ///
    /// This should get called when creating a new [`Supervisor`],
    /// [`Children`] or [`Child`] or resetting one of these.
    ///
    /// # Arguments
    ///
    /// * `parent` - The parent of this `Broadcast`.
    /// * `id` - The `BastionId` that should identify the new
    ///     `Broadcast`.
    ///
    /// [`Parent`]: /broadcast/enum.Parent.html
    /// [`BastionId`]: /context/struct.BastionId.html
    /// [`Supervisor`]: /supervisor/struct.Supervisor.html
    /// [`Children`]: /children/struct.Children.html
    /// [`Child`]: /children/struct.Child.html
    pub(crate) fn with_id(parent: Parent, id: BastionId) -> Self {
        let mut bcast = Broadcast::new(parent);
        bcast.id = id;

        bcast
    }

    pub(crate) fn id(&self) -> &BastionId {
        &self.id
    }

    pub(crate) fn sender(&self) -> &Sender {
        &self.sender
    }

    pub(crate) fn parent(&self) -> &Parent {
        &self.parent
    }

    /// Registers a `Broadcast` as a child, making it receive
    /// messages sent via [`send_children`] and [`stop_children`],
    /// [`kill_children`] and accessible via [`send_child`],
    /// [`stop_child`] and [`kill_child`].
    ///
    /// This **must** get called by [`System`] for every of its
    /// [`Supervisor`]s, by [`Supervisor`]s for every of their
    /// supervised elements and by [`Children`]s for every of
    /// their [`Child`].
    ///
    /// # Arguments
    ///
    /// * `child` - A `ref` to the `Broadcast` that should get
    ///     registered as a child.
    ///
    /// [`send_children`]: /broadcast/struct.Broadcast.html#method.send_children
    /// [`stop_children`]: /broadcast/struct.Broadcast.html#method.stop_children
    /// [`kill_children`]: /broadcast/struct.Broadcast.html#method.kill_children
    /// [`send_child`]: /broadcast/struct.Broadcast.html#method.send_child
    /// [`stop_child`]: /broadcast/struct.Broadcast.html#method.stop_child
    /// [`kill_child`]: /broadcast/struct.Broadcast.html#method.kill_child
    /// [`System`]: /system/struct.System.html
    /// [`Supervisor`]: /supervisor/struct.Supervisor.html
    /// [`Children`]: /children/struct.Children.html
    /// [`Child`]: /children/struct.Child.html
    pub(crate) fn register(&mut self, child: &Self) {
        self.children.insert(child.id.clone(), child.sender.clone());
    }

    /// Unregisters a registered child identified by its
    /// [`BastionId`].
    ///
    /// This **must** get called by [`System`] after stopping
    /// or killing a [`Supervisor`], by [`Supervisor`]s after
    /// stopping, killing or restarting a [`Children`] and by
    /// [`Children`]s after stopping or killing a [`Child`].
    ///
    /// # Arguments
    ///
    /// * `id` - The [`BastionId`] identifying the child that
    ///     should be unregistered.
    ///
    /// [`BastionId`]: /context/struct.BastionId.html
    /// [`System`]: /system/struct.System.html
    /// [`Supervisor`]: /supervisor/struct.Supervisor.html
    /// [`Children`]: /children/struct.Children.html
    /// [`Child`]: /children/struct.Child.html
    pub(crate) fn unregister(&mut self, id: &BastionId) {
        self.children.remove(id);
    }

    /// Unregisters all registered children.
    ///
    /// This **must** get called by the "system supervisor"
    /// when resetting (because others [`Supervisor`]s change
    /// their `Broadcast` but this one can't).
    ///
    /// [`Supervisor`]: /supervisor/struct.Supervisor.html
    pub(crate) fn clear_children(&mut self) {
        self.children.clear();
    }

    /// Sends a message to a registered child identified by
    /// its [`BastionId`] telling it to stop, and then
    /// [unregisters] it.
    ///
    /// # Arguments
    ///
    /// * `id` - The [`BastionId`] identifying the child that
    ///     should stop.
    ///
    /// [`BastionId`]: /context/struct.BastionId.html
    /// [unregisters]: /broadcast/struct.Broadcast.html#method.unregister
    pub(crate) fn stop_child(&mut self, id: &BastionId) {
        let msg = BastionMessage::stop();
        self.send_child(id, msg);

        self.unregister(id);
    }

    /// Sends a message to all registered children telling
    /// them to stop, and then [unregisters] them.
    ///
    /// [unregisters]: /broadcast/struct.Broadcast.html#method.clear_children
    pub(crate) fn stop_children(&mut self) {
        let msg = BastionMessage::stop();
        self.send_children(msg);

        self.clear_children();
    }

    /// Sends a message to a registered child identified by
    /// its [`BastionId`] telling it to kill itself, and then
    /// [unregisters] it.
    ///
    /// # Arguments
    ///
    /// * `id` - The [`BastionId`] identifying the child that
    ///     should get killed.
    ///
    /// [`BastionId`]: /context/struct.BastionId.html
    /// [unregisters]: /broadcast/struct.Broadcast.html#method.unregister
    pub(crate) fn kill_child(&mut self, id: &BastionId) {
        let msg = BastionMessage::kill();
        self.send_child(id, msg);

        self.unregister(id);
    }

    /// Sends a message to all registered children telling
    /// them to kill themselves, and then [unregisters] them.
    ///
    /// [unregisters]: /broadcast/struct.Broadcast.html#method.clear_children
    pub(crate) fn kill_children(&mut self) {
        let msg = BastionMessage::kill();
        self.send_children(msg);

        self.clear_children();
    }

    /// Sends a message saying that this `Broadcast` stopped
    /// to its parent.
    pub(crate) fn stopped(&mut self) {
        let msg = BastionMessage::stopped(self.id.clone());
        // FIXME: Err(msg)
        self.send_parent(msg).ok();
    }

    /// Sends a message saying that this `Broadcast` faulted
    /// to its parent.
    pub(crate) fn faulted(&mut self) {
        let msg = BastionMessage::faulted(self.id.clone());
        // FIXME: Err(msg)
        self.send_parent(msg).ok();
    }

    /// Sends a message to this `Broadcast`'s parent.
    ///
    /// # Arguments
    ///
    /// * `msg` - The message that should be sent.
    pub(crate) fn send_parent(&self, msg: BastionMessage) -> Result<(), BastionMessage> {
        self.parent.send(msg)
    }

    /// Sends a message to a registered child identified by
    /// its [`BastionId`].
    ///
    /// # Arguments
    ///
    /// * `msg` - The message that should be sent.
    ///
    /// [`BastionId`]: /context/struct.BastionId.html
    pub(crate) fn send_child(&self, id: &BastionId, msg: BastionMessage) {
        // FIXME: Err if None?
        if let Some(child) = self.children.get(id) {
            // FIXME: handle errors
            child.unbounded_send(msg).ok();
        }
    }

    /// Sends a message to all registered children.
    ///
    /// # Arguments
    ///
    /// * `msg` - The message that should be sent.
    pub(crate) fn send_children(&self, msg: BastionMessage) {
        for child in self.children.values() {
            // FIXME: Err(Error) if None
            if let Some(msg) = msg.try_clone() {
                // FIXME: handle errors
                child.unbounded_send(msg).ok();
            }
        }
    }

    /// Sends a message to this `Broadcast`.
    ///
    /// # Arguments
    ///
    /// * `msg` - The message that should be sent.
    pub(crate) fn send_self(&self, msg: BastionMessage) {
        // FIXME: handle errors
        self.sender.unbounded_send(msg).ok();
    }
}

impl Parent {
    /// Creates a new [`Parent::None`], which is only used by
    /// the [`System`] to indicate that it doesn't have a
    /// parent.
    ///
    /// [`Parent::None`]: /broadcast/enum.Parent.html#variant.None
    /// [`System`]: /system/struct.System.html
    pub(crate) fn none() -> Self {
        Parent::None
    }

    /// Creates a new [`Parent::System`], which is used by
    /// [`Supervisor`]s created via [`Bastion::supervisor`]
    /// and, thus, directly supervised by the [`System`].
    ///
    /// [`Parent::System`]: /broadcast/enum.Parent.html#variant.System
    /// [`Supervisor`]: /supervisor/struct.Supervisor.html
    /// [`Bastion::supervisor`]: /struct.Bastion.html#method.supervisor
    /// [`System`]: /system/struct.System.html
    pub(crate) fn system() -> Self {
        Parent::System
    }

    /// Creates a new [`Parent::Supervisor`], which is used
    /// by the [`Supervisor`]s created via
    /// [`Supervisor::supervisor`], [`Supervisor::supervisor_ref`]
    /// and [`SupervisorRef::supervisor`] and all [`Children`]s.
    ///
    /// # Arguments
    ///
    /// * `supervisor` - A [`SupervisorRef`] referencing the
    ///     supervisor supervising the element.
    ///
    /// [`Parent::Supervisor`]: /broadcast/enum.Parent.html#variant.Supervisor
    /// [`Supervisor`]: /supervisor/struct.Supervisor.html
    /// [`Supervisor::supervisor`]: /supervisor/struct.Supervisor.html#method.supervisor
    /// [`Supervisor::supervisor_ref`]: /supervisor/struct.Supervisor.html#method.supervisor
    /// [`SupervisorRef::supervisor`]: /supervisor/struct.SupervisorRef.html#method.supervisor
    /// [`Children`]: /children/struct.Children.html
    /// [`SupervisorRef`]: /supervisor/struct.SupervisorRef.html
    pub(crate) fn supervisor(supervisor: SupervisorRef) -> Self {
        Parent::Supervisor(supervisor)
    }

    /// Creates a new [`Parent::Children`], which is used
    /// by the [`Child`]s.
    ///
    /// # Arguments
    ///
    /// * `children` - A [`ChildrenRef`] referencing the
    ///     children group the `Child` belongs to.
    ///
    /// [`Parent::Children`]: /broadcast/enum.Parent.html#variant.Children
    /// [`Child`]: /children/struct.Child.html
    /// [`ChildrenRef`]: /children/struct.ChildrenRef.html
    pub(crate) fn children(children: ChildrenRef) -> Self {
        Parent::Children(children)
    }

    /// Consumes the `Parent`, returning a [`SupervisorRef`] if
    /// it is a [`Parent::Supervisor`] or `None` otherwise.
    ///
    /// [`SupervisorRef`]: /supervisor/struct.SupervisorRef.html
    /// [`Parent::Supervisor`]: /broadcast/enum.Parent.html#variant.Supervisor
    pub(crate) fn into_supervisor(self) -> Option<SupervisorRef> {
        if let Parent::Supervisor(supervisor) = self {
            Some(supervisor)
        } else {
            None
        }
    }

    /// Consumes the `Parent`, returning a [`ChildrenRef`] if
    /// it is a [`Parent::Children`] or `None` otherwise.
    ///
    /// [`ChildrenRef`]: /supervisor/struct.ChildrenRef.html
    /// [`Parent::Children`]: /broadcast/enum.Parent.html#variant.Children
    pub(crate) fn into_children(self) -> Option<ChildrenRef> {
        if let Parent::Children(children) = self {
            Some(children)
        } else {
            None
        }
    }

    /// Tries to send a message to this `Parent`.
    ///
    /// This method returns `()` if it succeeded, or `Err(msg)`
    /// otherwise.
    ///
    /// # Arguments
    ///
    /// * `msg` - The message to send.
    fn send(&self, msg: BastionMessage) -> Result<(), BastionMessage> {
        match self {
            // FIXME
            Parent::None => unimplemented!(),
            Parent::System => SYSTEM
                .sender()
                .unbounded_send(msg)
                .map_err(|err| err.into_inner()),
            Parent::Supervisor(supervisor) => supervisor.send(msg),
            Parent::Children(children) => children.send(msg),
        }
    }
}

impl Stream for Broadcast {
    type Item = BastionMessage;

    fn poll_next(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().recver).poll_next(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::{BastionMessage, Broadcast, Parent};
    use futures::executor;
    use futures::poll;
    use futures::prelude::*;
    use std::task::Poll;

    #[test]
    fn send_children() {
        let mut parent = Broadcast::new(Parent::none());

        let mut children = vec![];
        for _ in 0..4 {
            let child = Broadcast::new(Parent::none());
            parent.register(&child);

            children.push(child);
        }

        let msg = BastionMessage::start();

        parent.send_children(msg.try_clone().unwrap());
        executor::block_on(async {
            for child in &mut children {
                match poll!(child.next()) {
                    Poll::Ready(Some(BastionMessage::Start)) => (),
                    _ => panic!(),
                }
            }
        });

        parent.unregister(children[0].id());
        parent.send_children(msg.try_clone().unwrap());
        executor::block_on(async {
            assert!(poll!(children[0].next()).is_pending());

            for child in &mut children[1..] {
                match poll!(child.next()) {
                    Poll::Ready(Some(BastionMessage::Start)) => (),
                    _ => panic!(),
                }
            }
        });

        parent.clear_children();
        parent.send_children(msg);
        executor::block_on(async {
            for child in &mut children[1..] {
                assert!(poll!(child.next()).is_pending());
            }
        });
    }
}
