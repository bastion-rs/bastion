use crate::child::{BastionChildren, BastionClosure, Message};
use crate::messages::PoisonPill;
use crate::spawn::RuntimeSpawn;
use crate::supervisor::Supervisor;
use crossbeam_channel::{unbounded, Receiver, Sender};
use ratelimit::Limiter;
use std::any::Any;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::panic::AssertUnwindSafe;
use std::time::Duration;
use tokio::prelude::future::FutureResult;
use tokio::prelude::*;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct BastionContext {
    pub parent: Option<Box<Supervisor>>,
    pub descendants: Vec<BastionChildren>,
    pub killed: Vec<BastionChildren>,
    pub bcast_tx: Option<Sender<Box<dyn Message>>>,
    pub bcast_rx: Option<Receiver<Box<dyn Message>>>,
}

impl BastionContext {
    fn dispatch_clock() -> Limiter {
        ratelimit::Builder::new()
            .capacity(1)
            .quantum(1)
            .interval(Duration::new(0, 100))
            .build()
    }

    pub fn hook(self) {
        let mut dc = BastionContext::dispatch_clock();
        dc.wait();
        let rx = self.bcast_rx.clone().unwrap();
        if let Ok(message) = rx.try_recv() {
            let msg: &dyn Any = message.as_any();
            if msg.is::<PoisonPill>() {
                dc.wait();
                panic!("PoisonPill");
            }
        }
    }

    pub fn blocking_hook(self) {
        let mut dc = BastionContext::dispatch_clock();
        dc.wait();
        let rx = self.bcast_rx.clone().unwrap();
        loop {
            if let Ok(message) = rx.try_recv() {
                let msg: &dyn Any = message.as_any();
                if msg.is::<PoisonPill>() {
                    dc.wait();
                    panic!("PoisonPill");
                }
            }
        }
    }

    pub fn spawn<F, M>(mut self, thunk: F, msg: M, scale: i32) -> Self
    where
        F: BastionClosure,
        M: Message,
    {
        let nt = Box::new(thunk);
        let msg_box = Box::new(msg);
        let current_spv = self.parent.clone().unwrap();
        let (p, c) = (current_spv.ctx.bcast_tx, current_spv.ctx.bcast_rx);

        for child_id in 0..scale {
            let children = BastionChildren {
                id: Uuid::new_v4().to_string(),
                tx: p.clone(),
                rx: c.clone(),
                redundancy: scale,
                msg: objekt::clone_box(&*msg_box),
                thunk: objekt::clone_box(&*nt),
            };

            let mut this_spv = *self.parent.clone().unwrap();
            this_spv.ctx.descendants.push(children.clone());

            let tx = children.tx.as_ref().unwrap().clone();
            let rx = children.rx.clone().unwrap();

            let nt = objekt::clone_box(&*children.thunk);
            let msgr = objekt::clone_box(&*children.msg);
            let msgr_panic_handler = objekt::clone_box(&*children.msg);
            let mut if_killed = children.clone();

            let context_spv = this_spv.clone();
            if_killed.id = format!(
                "{}::{}::{}",
                context_spv.clone().urn.name,
                if_killed.id,
                child_id
            );

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

        self
    }
}

impl Debug for BastionContext {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "\nContext\n\tParent :: {:?}, Descendants :: {:?}, Killed :: {:?}, TX :: {:?}, RX :: {:?}\n\t",
            self.parent, self.descendants, self.killed, self.bcast_tx, self.bcast_rx
        )
    }
}
