use crate::child::Message;
use crate::messages::PoisonPill;
use crossbeam_channel::{Receiver, Sender};
use ratelimit::Limiter;
use std::any::Any;
use std::time::Duration;

#[derive(Clone)]
pub struct BastionContext {
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
}
