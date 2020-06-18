use crate::children_ref::ChildrenRef;
use crate::Bastion;
use crate::context::*;
use crate::message::{BastionMessage, Message};
use crate::envelope::{RefAddr, SignedMessage};
use crate::message::Msg;

use std::sync::Arc;
use artillery_core::cluster::ap::*;
use artillery_core::epidemic::prelude::*;

use tracing::*;
use futures::future;
use core::future::Future;
use futures::pending;
use futures::future::FutureExt;
use crate::child::Exec;
use lever::table::lotable::*;

use uuid::Uuid;

#[derive(Debug)]
pub struct ClusterMessage {
    pub(crate) msg: Msg,
    pub(crate) member: Uuid,
}

impl ClusterMessage {
    pub fn new(msg: Msg, member: Uuid) -> Self {
        ClusterMessage { msg, member }
    }

    pub fn extract(self) -> Msg
    {
        self.msg
    }
}

pub struct DistributedContext {
    bctx: BastionContext,
    me: Uuid,
    members: LOTable<Uuid, ArtilleryMember>,
    cluster: Arc<Cluster>
}

impl DistributedContext {
    fn new(bctx: BastionContext, cluster: Arc<Cluster>, me: Uuid) -> Self {
        DistributedContext {
            bctx,
            me,
            members: LOTable::new(),
            cluster
        }
    }

    fn local_ctx(&self) -> &BastionContext {
        &self.bctx
    }

    pub fn current(&self) -> Uuid {
        self.me
    }

    pub fn members(&self) -> Vec<ArtilleryMember> {
        self.members.values()
            .into_iter()
            .filter(|m| m.host_key() != self.me)
            .collect()
    }

    pub fn tell<M>(&self, to: &Uuid, msg: M) -> Result<(), M>
    where
        M: Message + AsRef<str>
    {
        debug!("Sending payload");
        self.cluster.send_payload(*to, msg);
        Ok(())
    }

    pub async fn recv(&self) -> Result<ClusterMessage, ()> {
        debug!("DistributedContext({}): Waiting to receive message.", self.me);
        loop {
            for (members, event) in self.cluster.events.try_iter() {
                warn!(event = format!("{:?}", event).as_str(), "Cluster event");
                match event {
                    ArtilleryMemberEvent::Payload(member, msg) => {
                        return Ok(ClusterMessage::new(Msg::tell(msg), member.host_key()));
                    },
                    _ => {}
                }

                members
                    .iter()
                    .for_each(|m| {
                        match m.state() {
                            ArtilleryMemberState::Alive => {
                                let _ = self.members.insert(m.host_key(), m.clone());
                            },
                            ArtilleryMemberState::Down => {
                                let _ = self.members.remove(&m.host_key());
                            },
                            _ => {}
                        }
                    });
            }
        }
    }
}


///
/// Creates distributed cluster actor
pub fn cluster_actor<I, F>(
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
        let dctx = Arc::new(DistributedContext::new(ctx, ap_cluster.cluster(), cluster_config.node_id));
        let action = action.clone();

        let core = async move {
            let ap_events = ap_cluster.clone();

            // Detach cluster launch
            let cluster_handle =
                blocking!(ap_cluster.launch().await);

            let events_handle = blocking!(
                action(dctx).await;
            );

            future::join(events_handle, cluster_handle).await
        };

        async move {
            run!(core);
            Ok(())
        }
    })
}
