use crate::child::{BastionChildren, BastionClosure, Message};
use crate::context::BastionContext;
use crossbeam_channel::unbounded;
use std::cmp::Ordering;
use std::panic::AssertUnwindSafe;
use tokio::prelude::future::FutureResult;
use tokio::prelude::*;
use uuid::Uuid;

///
/// Identifier struct for Supervisor
/// System uses this to assemble resource name for children and supervisors.
#[derive(Clone, PartialOrd, PartialEq, Eq, Debug)]
pub struct SupervisorURN {
    pub sys: String,  // Supervisor System Name
    pub name: String, // Supervisor Name
    pub res: String,  // Supervisor Identifier
}

impl Default for SupervisorURN {
    fn default() -> Self {
        let uuid_gen = Uuid::new_v4();
        SupervisorURN {
            sys: "bastion".to_owned(),
            name: "default-supervisor".to_owned(),
            res: uuid_gen.to_string(),
        }
    }
}

impl Ord for SupervisorURN {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sys
            .cmp(&other.sys)
            .then(self.name.cmp(&other.name))
            .then(self.res.cmp(&other.res))
    }
}

///
/// Possible supervision strategies to pass to the supervisor.
///
/// **OneForOne**: If a child gets killed only that child will be restarted under the supervisor.
///
/// **OneForAll**: If a child gets killed all children at the same level under the supervision will be restarted.
///
/// **RestForOne**: If a child gets killed restart the rest of the children at the same level under the supervisor.
#[derive(Clone, Debug)]
pub enum SupervisionStrategy {
    /// If a child gets killed only that child will be restarted under the supervisor.
    ///
    /// Example is from [learnyousomeerlang.com](https://learnyousomeerlang.com):
    ///
    /// ![](https://learnyousomeerlang.com/static/img/restart-one-for-one.png)
    OneForOne,
    /// If a child gets killed all children at the same level under the supervision will be restarted.
    ///
    /// Example is from [learnyousomeerlang.com](https://learnyousomeerlang.com):
    ///
    /// ![](https://learnyousomeerlang.com/static/img/restart-one-for-all.png)
    OneForAll,
    /// If a child gets killed restart the rest of the children at the same level under the supervisor.
    ///
    /// Example is from [learnyousomeerlang.com](https://learnyousomeerlang.com):
    ///
    /// ![](https://learnyousomeerlang.com/static/img/restart-rest-for-one.png)
    RestForOne,
}

impl Default for SupervisionStrategy {
    fn default() -> Self {
        SupervisionStrategy::OneForOne
    }
}

#[derive(Default, Clone, Debug)]
pub struct Supervisor {
    pub urn: SupervisorURN,
    pub(crate) ctx: BastionContext,
    pub(crate) strategy: SupervisionStrategy,
}

impl Supervisor {
    pub fn props(mut self, name: String, system: String) -> Self {
        let mut urn = SupervisorURN::default();
        urn.name = name;
        self.urn = urn;
        self.urn.sys = system;
        self
    }

    pub fn strategy(mut self, strategy: SupervisionStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn children<F, M>(mut self, thunk: F, msg: M, scale: i32) -> Self
    where
        F: BastionClosure,
        M: Message,
    {
        let bt = Box::new(thunk);
        let msg_box = Box::new(msg);
        let (p, c) = unbounded();

        let children = BastionChildren {
            id: Uuid::new_v4().to_string(),
            tx: Some(p.clone()),
            rx: Some(c.clone()),
            redundancy: scale,
            msg: objekt::clone_box(&*msg_box),
            thunk: objekt::clone_box(&*bt),
        };

        self.ctx.descendants.push(children);
        self.ctx.bcast_rx = Some(c.clone());
        self.ctx.bcast_tx = Some(p.clone());
        self.ctx.parent = Some(Box::new(self.clone()));

        self
    }

    pub fn launch(mut self) {
        for descendant in &self.ctx.descendants {
            let descendant = descendant.clone();

            for child_id in 0..descendant.redundancy {
                let tx = descendant.tx.as_ref().unwrap().clone();
                let rx = descendant.rx.clone().unwrap();

                let nt = objekt::clone_box(&*descendant.thunk);
                let msgr = objekt::clone_box(&*descendant.msg);
                let msgr_panic_handler = objekt::clone_box(&*descendant.msg);
                let mut if_killed = descendant.clone();
                if_killed.id = format!("{}::{}", if_killed.id, child_id);

                let mut this_spv = self.clone();
                let context_spv = self.clone();

                let f = future::lazy(move || {
                    nt(
                        BastionContext {
                            parent: Some(Box::new(context_spv.clone())),
                            descendants: context_spv.ctx.descendants,
                            killed: context_spv.ctx.killed,
                            bcast_rx: Some(rx.clone()),
                            bcast_tx: Some(tx.clone()),
                        },
                        msgr,
                    );
                    future::ok::<(), ()>(())
                });

                let k = AssertUnwindSafe(f)
                    .catch_unwind()
                    .then(|result| -> FutureResult<(), ()> {
                        this_spv.ctx.killed.push(if_killed);

                        // Already re-entrant code
                        if let Err(err) = result {
                            error!("Panic happened in supervised child - {:?}", err);
                            crate::bastion::Bastion::fault_recovery(this_spv, msgr_panic_handler);
                        }
                        future::ok(())
                    });

                let ark = crate::bastion::PLATFORM.clone();
                let mut runtime = ark.lock();
                let shared_runtime = &mut runtime.runtime;
                shared_runtime.spawn(k);
            }
        }


        // FIXME: There might be discrepancy between passed self and referenced self.
        // Fix this with either passing reference without Box (lifetimes sigh!)
        // Or use channels to send back to the supervision tree.
        self.ctx.parent = Some(Box::new(self.clone()));
    }
}
