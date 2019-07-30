use crate::child::{BastionChildren, BastionClosure, Message};
use crate::context::BastionContext;
use crossbeam_channel::unbounded;
use std::cmp::Ordering;
use std::panic::AssertUnwindSafe;
use tokio::prelude::future::FutureResult;
use tokio::prelude::*;
use uuid::Uuid;

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

#[derive(Clone, Debug)]
pub enum SupervisionStrategy {
    OneForOne,
    OneForAll,
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
    pub(crate) descendants: Vec<BastionChildren>,
    pub(crate) killed: Vec<BastionChildren>,
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
            tx: Some(p),
            rx: Some(c),
            redundancy: scale,
            msg: objekt::clone_box(&*msg_box),
            thunk: objekt::clone_box(&*bt),
        };

        self.descendants.push(children);

        self
    }

    pub fn launch(self) {
        for descendant in &self.descendants {
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
                            spv: Some(context_spv),
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
                        this_spv.killed.push(if_killed);

                        // Already re-entrant code
                        if let Err(err) = result {
                            error!("Panic happened in supervised child - {:?}", err);
                            crate::bastion::Bastion::fault_recovery(this_spv, msgr_panic_handler);
                        }
                        future::ok(())
                    });

                let ark = crate::bastion::PLATFORM.clone();
                let mut runtime = ark.lock().unwrap();
                let shared_runtime = &mut runtime.runtime;
                shared_runtime.spawn(k);
            }
        }
    }
}
