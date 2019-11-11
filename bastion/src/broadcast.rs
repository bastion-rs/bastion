use crate::children::ChildrenRef;
use crate::context::BastionId;
use crate::message::BastionMessage;
use crate::supervisor::SupervisorRef;
use crate::system::SYSTEM_SENDER;
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
    // Used by the `System`.
    None,
    // Used by the `Supervisor`s started via `Bastion::supervisor`.
    System,
    // Used by the `Supervisor`s started via `Supervisor::supervisor`,
    // `Supervisor::supervisor_ref` and `SupervisorRef::supervisor` and
    // all the `Children`s.
    Supervisor(SupervisorRef),
    // Used by the `Child`s.
    Children(ChildrenRef),
}

impl Broadcast {
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
    /// [`send_children`]: #method.send_children
    /// [`stop_children`]: #method.stop_children
    /// [`kill_children`]: #method.kill_children
    /// [`send_child`]: #method.send_child
    /// [`stop_child`]: #method.stop_child
    /// [`kill_child`]: #method.kill_child
    /// [`System`]: system/struct.System.html
    /// [`Supervisor`]: supervisor/struct.Supervisor.html
    /// [`Children`]: children/struct.Children.html
    /// [`Child`]: children/struct.Child.html
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
    /// [`BastionId`]: context/struct.BastionId.html
    /// [`System`]: system/struct.System.html
    /// [`Supervisor`]: supervisor/struct.Supervisor.html
    /// [`Children`]: children/struct.Children.html
    /// [`Child`]: children/struct.Child.html
    pub(crate) fn unregister(&mut self, id: &BastionId) {
        self.children.remove(id);
    }

    /// Unregisters all registered children.
    ///
    /// This **must** get called by the "system supervisor"
    /// when resetting (because others [`Supervisor`]s change
    /// their `Broadcast` but this one can't).
    ///
    /// [`Supervisor`]: supervisor/struct.Supervisor.html
    pub(crate) fn clear_children(&mut self) {
        self.children.clear();
    }

    /// Sends a message to a registered child identified by
    /// its [`BastionId`] telling it to stop, and then
    /// [unregisters] it.
    ///
    /// [`BastionId`]: context/struct.BastionId.html
    /// [unregisters]: #method.unregister
    pub(crate) fn stop_child(&mut self, id: &BastionId) {
        let msg = BastionMessage::stop();
        self.send_child(id, msg);

        self.unregister(id);
    }

    /// Sends a message to all registered children telling
    /// them to stop, and then [unregisters] them.
    ///
    /// [unregisters]: #method.clear_children
    pub(crate) fn stop_children(&mut self) {
        let msg = BastionMessage::stop();
        self.send_children(msg);

        self.clear_children();
    }

    /// Sends a message to a registered child identified by
    /// its [`BastionId`] telling it to kill itself, and then
    /// [unregisters] it.
    ///
    /// [`BastionId`]: context/struct.BastionId.html
    /// [unregisters]: #method.unregister
    pub(crate) fn kill_child(&mut self, id: &BastionId) {
        let msg = BastionMessage::kill();
        self.send_child(id, msg);

        self.unregister(id);
    }

    /// Sends a message to all registered children telling
    /// them to kill themselves, and then [unregisters] them.
    ///
    /// [unregisters]: #method.clear_children
    pub(crate) fn kill_children(&mut self) {
        let msg = BastionMessage::kill();
        self.send_children(msg);

        self.clear_children();
    }

    /// Sends a message saying that this `Broadcast` stopped
    /// to its parent.
    pub(crate) fn stopped(&mut self) {
        self.stop_children();

        let msg = BastionMessage::stopped(self.id.clone());
        // FIXME: Err(msg)
        self.send_parent(msg).ok();
    }

    /// Sends a message saying that this `Broadcast` faulted
    /// to its parent.
    pub(crate) fn faulted(&mut self) {
        self.kill_children();

        let msg = BastionMessage::faulted(self.id.clone());
        // FIXME: Err(msg)
        self.send_parent(msg).ok();
    }

    /// Sends a message to this `Broadcast`'s parent.
    pub(crate) fn send_parent(&self, msg: BastionMessage) -> Result<(), BastionMessage> {
        self.parent.send(msg)
    }

    /// Sends a message to a registered child identified by
    /// its [`BastionId`].
    ///
    /// [`BastionId`]: context/struct.BastionId.html
    pub(crate) fn send_child(&self, id: &BastionId, msg: BastionMessage) {
        // FIXME: Err if None?
        if let Some(child) = self.children.get(id) {
            // FIXME: handle errors
            child.unbounded_send(msg).ok();
        }
    }

    /// Sends a message to all registered children.
    pub(crate) fn send_children(&self, msg: BastionMessage) {
        for (_, child) in &self.children {
            // FIXME: Err(Error) if None
            if let Some(msg) = msg.try_clone() {
                // FIXME: handle errors
                child.unbounded_send(msg).ok();
            }
        }
    }

    /// Sends a message to this `Broadcast`.
    pub(crate) fn send_self(&self, msg: BastionMessage) {
        // FIXME: handle errors
        self.sender.unbounded_send(msg).ok();
    }
}

impl Parent {
    pub(crate) fn none() -> Self {
        Parent::None
    }

    pub(crate) fn system() -> Self {
        Parent::System
    }

    pub(crate) fn supervisor(supervisor: SupervisorRef) -> Self {
        Parent::Supervisor(supervisor)
    }

    pub(crate) fn children(children: ChildrenRef) -> Self {
        Parent::Children(children)
    }

    pub(crate) fn into_supervisor(self) -> Option<SupervisorRef> {
        if let Parent::Supervisor(supervisor) = self {
            Some(supervisor)
        } else {
            None
        }
    }

    pub(crate) fn into_children(self) -> Option<ChildrenRef> {
        if let Parent::Children(children) = self {
            Some(children)
        } else {
            None
        }
    }

    fn send(&self, msg: BastionMessage) -> Result<(), BastionMessage> {
        match self {
            // FIXME
            Parent::None => unimplemented!(),
            Parent::System => SYSTEM_SENDER
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
