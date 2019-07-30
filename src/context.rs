use crate::child::{Message, BastionChildren, BastionClosure};
use crate::messages::PoisonPill;
use crossbeam_channel::{Receiver, Sender, unbounded};
use ratelimit::Limiter;
use std::any::Any;
use std::time::Duration;
use crate::spawn::RuntimeSpawn;
use crate::supervisor::Supervisor;
use uuid::Uuid;

#[derive(Clone)]
pub struct BastionContext {
    pub spv: Option<Supervisor>,
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

//        self.descendants.push(children);

        self
    }
}
