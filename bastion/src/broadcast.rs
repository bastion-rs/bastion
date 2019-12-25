use crate::children_ref::ChildrenRef;
use crate::context::BastionId;
use crate::envelope::Envelope;
use crate::message::BastionMessage;
use crate::path::{BastionPath, BastionPathElement};
use crate::supervisor::SupervisorRef;
use crate::system::SYSTEM;
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::prelude::*;
use fxhash::FxHashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

pub(crate) type Sender = UnboundedSender<Envelope>;
pub(crate) type Receiver = UnboundedReceiver<Envelope>;

#[derive(Debug)]
pub(crate) struct Broadcast {
    sender: Sender,
    recver: Receiver,
    path: Arc<BastionPath>, // Arc is needed because we put path to Envelope
    parent: Parent,
    children: FxHashMap<BastionId, Sender>,
}

#[derive(Debug, Clone)]
pub(crate) enum Parent {
    None,
    System,
    Supervisor(SupervisorRef),
    Children(ChildrenRef),
}

impl Parent {
    pub(super) fn is_none(&self) -> bool {
        match self {
            Parent::None => true,
            _ => false,
        }
    }

    pub(super) fn is_system(&self) -> bool {
        match self {
            Parent::System => true,
            _ => false,
        }
    }
}

impl Broadcast {
    pub(crate) fn new(parent: Parent, element: BastionPathElement) -> Self {
        Self::new_with_channel(parent, element, mpsc::unbounded())
    }

    pub(crate) fn new_with_channel(parent: Parent, element: BastionPathElement, channel: (Sender, Receiver)) -> Self {
        let children = FxHashMap::default();

        let parent_path: BastionPath = match &parent {
            Parent::None | Parent::System => BastionPath::root(),
            Parent::Supervisor(sv_ref) => BastionPath::clone(sv_ref.path()),
            Parent::Children(ch_ref) => BastionPath::clone(ch_ref.path()),
        };

        // FIXME: unwrap
        let path = parent_path
            .append(element)
            .expect("Can't append path in Broadcast::new");
        let path = Arc::new(path);

        Broadcast {
            parent,
            sender: channel.0,
            recver: channel.1,
            path,
            children,
        }
    }

    pub(crate) fn new_root(parent: Parent) -> Self {
        // FIXME
        assert!(parent.is_none() || parent.is_system());

        let (sender, recver) = mpsc::unbounded();
        let children = FxHashMap::default();
        let path = BastionPath::root();
        let path = Arc::new(path);

        Broadcast {
            parent,
            sender,
            recver,
            path,
            children,
        }
    }

    pub(crate) fn extract_channel(self) -> (Sender, Receiver) {
        (self.sender, self.recver)
    }

    pub(crate) fn id(&self) -> &BastionId {
        self.path.id()
    }

    pub(crate) fn sender(&self) -> &Sender {
        &self.sender
    }

    pub(crate) fn path(&self) -> &Arc<BastionPath> {
        &self.path
    }

    pub(crate) fn parent(&self) -> &Parent {
        &self.parent
    }

    pub(crate) fn register(&mut self, child: &Self) {
        self.children
            .insert(child.id().clone(), child.sender.clone());
    }

    pub(crate) fn unregister(&mut self, id: &BastionId) {
        self.children.remove(id);
    }

    pub(crate) fn clear_children(&mut self) {
        self.children.clear();
    }

    pub(crate) fn stop_child(&mut self, id: &BastionId) {
        let msg = BastionMessage::stop();
        let env = Envelope::new(msg, self.path.clone(), self.sender.clone());
        self.send_child(id, env);

        self.unregister(id);
    }

    pub(crate) fn stop_children(&mut self) {
        let msg = BastionMessage::stop();
        let env = Envelope::new(msg, self.path.clone(), self.sender.clone());
        self.send_children(env);

        self.clear_children();
    }

    pub(crate) fn kill_child(&mut self, id: &BastionId) {
        let msg = BastionMessage::kill();
        let env = Envelope::new(msg, self.path.clone(), self.sender.clone());
        self.send_child(id, env);

        self.unregister(id);
    }

    pub(crate) fn kill_children(&mut self) {
        let msg = BastionMessage::kill();
        let env = Envelope::new(msg, self.path.clone(), self.sender.clone());
        self.send_children(env);

        self.clear_children();
    }

    pub(crate) fn stopped(&mut self) {
        self.stop_children();

        let msg = BastionMessage::stopped(self.id().clone());
        let env = Envelope::new(msg, self.path.clone(), self.sender.clone());
        // FIXME: Err(msg)
        self.send_parent(env).ok();
    }

    pub(crate) fn faulted(&mut self) {
        self.kill_children();

        let msg = BastionMessage::faulted(self.id().clone());
        let env = Envelope::new(msg, self.path.clone(), self.sender.clone());
        // FIXME: Err(msg)
        self.send_parent(env).ok();
    }

    pub(crate) fn send_parent(&self, envelope: Envelope) -> Result<(), Envelope> {
        self.parent.send(envelope)
    }

    pub(crate) fn send_child(&self, id: &BastionId, envelope: Envelope) {
        // FIXME: Err if None?
        if let Some(child) = self.children.get(id) {
            // FIXME: handle errors
            child.unbounded_send(envelope).ok();
        }
    }

    pub(crate) fn send_children(&self, env: Envelope) {
        for child in self.children.values() {
            // FIXME: Err(Error) if None
            if let Some(s_env) = env.try_clone() {
                // FIXME: handle errors
                child.unbounded_send(s_env).map_err(|e| {
                    error!("Unable to send_children: {:?} {:?}", env, e);
                }).ok();
            }
        }
    }

    pub(crate) fn send_self(&self, env: Envelope) {
        // FIXME: handle errors
        self.sender.unbounded_send(env).ok();
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

    fn send(&self, env: Envelope) -> Result<(), Envelope> {
        match self {
            // FIXME
            Parent::None => unimplemented!(),
            Parent::System => SYSTEM
                .sender()
                .unbounded_send(env)
                .map_err(|err| err.into_inner()),
            Parent::Supervisor(supervisor) => supervisor.send(env),
            Parent::Children(children) => children.send(env),
        }
    }
}

impl Stream for Broadcast {
    type Item = Envelope;

    fn poll_next(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().recver).poll_next(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::{BastionMessage, Broadcast, Parent};
    use crate::context::{BastionId, NIL_ID};
    use crate::envelope::Envelope;
    use crate::path::{BastionPath, BastionPathElement};
    use futures::channel::mpsc;
    use futures::executor;
    use futures::poll;
    use futures::prelude::*;
    use std::sync::Arc;
    use std::task::Poll;

    #[test]
    fn send_children() {
        let mut parent = Broadcast::new_root(Parent::System);

        let mut children = vec![];
        for _ in 0..4 {
            let child = Broadcast::new(
                Parent::System,
                BastionPathElement::Supervisor(BastionId::new()),
            );
            parent.register(&child);
            children.push(child);
        }

        let msg = BastionMessage::start();

        // need manual construction because SYSTEM is not running in this test
        let (sender, _) = mpsc::unbounded();
        let env = Envelope::new(
            msg,
            Arc::new(
                BastionPath::root()
                    .append(BastionPathElement::Supervisor(NIL_ID))
                    .unwrap()
                    .append(BastionPathElement::Children(NIL_ID))
                    .unwrap(),
            ),
            sender,
        );

        parent.send_children(env.try_clone().unwrap());
        executor::block_on(async {
            for child in &mut children {
                match poll!(child.next()) {
                    Poll::Ready(Some(Envelope {
                        msg: BastionMessage::Start,
                        ..
                    })) => (),
                    _ => panic!(),
                }
            }
        });

        parent.unregister(children[0].id());
        parent.send_children(env.try_clone().unwrap());
        executor::block_on(async {
            assert!(poll!(children[0].next()).is_pending());

            for child in &mut children[1..] {
                match poll!(child.next()) {
                    Poll::Ready(Some(Envelope {
                        msg: BastionMessage::Start,
                        ..
                    })) => (),
                    _ => panic!(),
                }
            }
        });

        parent.clear_children();
        parent.send_children(env);
        executor::block_on(async {
            for child in &mut children[1..] {
                assert!(poll!(child.next()).is_pending());
            }
        });
    }
}
