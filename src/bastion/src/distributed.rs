//!
//! Cluster formation and distributed actor instantiation
use crate::children_ref::ChildrenRef;
use crate::context::*;
use crate::message::Message;
use crate::Bastion;

use crate::message::Msg;

use artillery_core::cluster::ap::*;
use artillery_core::epidemic::prelude::*;
use std::sync::Arc;

use core::future::Future;
use futures::future;
use tracing::*;

use lever::table::lotable::*;

use uuid::Uuid;

///
/// Cluster message that is sent and delivered among members
#[derive(Debug)]
pub struct ClusterMessage {
    pub(crate) msg: Msg,
    pub(crate) member: Uuid,
}

impl ClusterMessage {
    ///
    /// Create a `ClusterMessage` from a `Msg` and a member
    pub fn new(msg: Msg, member: Uuid) -> Self {
        ClusterMessage { msg, member }
    }

    ///
    /// Extract a `Msg` from a `ClusterMessage`
    pub fn extract(self) -> Msg {
        self.msg
    }
}

///
/// Distributed context that holds currently formed/forming cluster's context.
#[derive(Debug)]
pub struct DistributedContext {
    bctx: BastionContext,
    me: Uuid,
    members: LOTable<Uuid, ArtilleryMember>,
    cluster: Arc<Cluster>,
}

impl DistributedContext {
    ///
    /// Initializes distributed context with underlying actor's local context and cluster handle.
    fn new(bctx: BastionContext, cluster: Arc<Cluster>, me: Uuid) -> Self {
        DistributedContext {
            bctx,
            me,
            members: LOTable::new(),
            cluster,
        }
    }

    ///
    /// Exposes local bastion context to distributed actor
    pub fn local_ctx(&self) -> &BastionContext {
        &self.bctx
    }

    ///
    /// Gets the current member's node id in the cluster
    ///
    /// This shouldn't be confused with content addressing of actors in the application.
    pub fn current(&self) -> Uuid {
        self.me
    }

    ///
    /// Get current members of the cluster.
    ///
    /// This list is continuously updates with cluster state.
    /// If a node is down it won't appear in this list.
    /// If a node is suspected, it will still be in here. When suspected message delivery isn't guaranteed.
    pub fn members(&self) -> Vec<ArtilleryMember> {
        self.members
            .values()
            .filter(|m| m.host_key() != self.me)
            .collect()
    }

    ///
    /// Send a fire and forget style message to a destined cluster member.
    /// Message needs to be stringified or apply the rules of bastion's [Message] trait.
    pub fn tell<M>(&self, to: &Uuid, msg: M) -> Result<(), M>
    where
        M: Message + AsRef<str>,
    {
        debug!("Sending payload");
        self.cluster.send_payload(*to, msg);
        Ok(())
    }

    ///
    /// Channel that aggregates incoming cluster events to this node.
    pub async fn recv(&self) -> Result<ClusterMessage, ()> {
        debug!(
            "DistributedContext({}): Waiting to receive message.",
            self.me
        );
        loop {
            for (members, event) in self.cluster.events.try_iter() {
                warn!(event = format!("{:?}", event).as_str(), "Cluster event");
                if let ArtilleryMemberEvent::Payload(member, msg) = event {
                    return Ok(ClusterMessage::new(Msg::tell(msg), member.host_key()));
                }

                members.iter().for_each(|m| match m.state() {
                    ArtilleryMemberState::Alive => {
                        let _ = self.members.insert(m.host_key(), m.clone());
                    }
                    ArtilleryMemberState::Down => {
                        let _ = self.members.remove(&m.host_key());
                    }
                    _ => {}
                });
            }
        }
    }
}

///
/// Creates distributed cluster actor
pub(crate) fn cluster_actor<I, F>(
    cluster_config: &'static ArtilleryAPClusterConfig,
    action: I,
) -> Result<ChildrenRef, ()>
where
    I: Fn(Arc<DistributedContext>) -> F + Send + Sync + 'static,
    F: Future<Output = Result<(), ()>> + Send + 'static,
{
    let action = Arc::new(action);

    Bastion::spawn(move |ctx: BastionContext| {
        let ap_cluster = Arc::new(ArtilleryAPCluster::new(cluster_config.clone()).unwrap());
        let dctx = Arc::new(DistributedContext::new(
            ctx,
            ap_cluster.cluster(),
            cluster_config.node_id,
        ));
        let action = action.clone();

        let core = async move {
            let _ap_events = ap_cluster.clone();

            // Detach cluster launch
            let cluster_handle = blocking!(ap_cluster.launch().await);

            let events_handle = blocking!(action(dctx).await);

            future::join(events_handle, cluster_handle).await
        };

        async move {
            run!(core);
            Ok(())
        }
    })
}
