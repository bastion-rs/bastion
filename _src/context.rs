//!
//! Context is generic structure to share resource between supervisor, children and the system.
//!
//! This enable us to work easily with system messages, api exposure and communication between processes.
//!
//! BastionContext is the main primitive to share this data anything related to internal
//! communication is here.

use crate::child::{BastionChildren, BastionClosure, Message};
use crate::messages::PoisonPill;
use crate::supervisor::Supervisor;
use crossbeam_channel::{Receiver, Sender};
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

///
/// Context definition for any lightweight process.
///
/// You can use context to:
/// * spawn children
/// * communicate with other spawned processes
/// * communicate with parent
///
/// It is used internally for:
/// * communication with the system
/// * tracking the supervision
#[derive(Clone, Default)]
pub struct BastionContext {
    /// Reference to the parent, it is [None] for root supervisor.
    pub parent: Option<Box<Supervisor>>,

    /// Container holding children.
    pub descendants: Vec<BastionChildren>,

    /// Container holding killed children.
    pub(crate) killed: Vec<BastionChildren>,

    /// Send endpoint for system broadcast.
    pub bcast_tx: Option<Sender<Box<dyn Message>>>,

    /// Receive endpoint for system broadcast.
    pub bcast_rx: Option<Receiver<Box<dyn Message>>>,
}

impl BastionContext {
    /// Prevents thundering-herd effect while system message broadcasts are
    /// over the expected amount.
    fn dispatch_clock() -> Limiter {
        ratelimit::Builder::new()
            .capacity(1)
            .quantum(1)
            .interval(Duration::new(0, 100))
            .build()
    }

    ///
    /// One-time use hook for spawned processes.
    /// You need to have this function for processes which
    /// you want to end gracefully by the system after
    /// successful completion.
    ///
    /// # Examples
    /// ```
    /// use bastion::prelude::*;
    ///
    /// fn main() {
    ///     Bastion::platform();
    ///     Bastion::spawn(|context: BastionContext, _msg: Box<dyn Message>| {
    ///         println!("root supervisor - spawn_at_root - 1");
    ///
    ///         // Rebind to the system
    ///         context.hook();
    ///     }, String::from("A Message"));
    ///
    ///     // Comment out to start the system, so runtime can initialize.
    ///     // Bastion::start()
    /// }
    /// ```
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

    ///
    /// Forever running hook for spawned processes.
    /// You need to have this function for processes which
    /// you want to reutilize the process later and handover the control
    /// back again to the system after successful completion.
    ///
    /// This function is a must to use for receiving signals from supervision and
    /// applying supervision strategies.
    ///
    /// # Examples
    /// ```
    /// use bastion::prelude::*;
    ///
    /// fn main() {
    ///     Bastion::platform();
    ///     Bastion::spawn(|context: BastionContext, _msg: Box<dyn Message>| {
    ///         println!("root supervisor - spawn_at_root - 1");
    ///
    ///         // Rebind to the system
    ///         context.blocking_hook();
    ///     }, String::from("A Message"));
    ///
    ///     // Comment out to start the system, so runtime can initialize.
    ///     // Bastion::start()
    /// }
    /// ```
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

    ///
    /// Context level spawn function for child generation from the parent context.
    /// This context carries global broadcast for the system.
    /// Every context directly tied to the parent process.
    /// If you listen broadcast tx/rx pair in the parent process,
    /// you can communicate with the children with specific message type.
    ///
    /// Bastion doesn't enforce you to use specific Message type or force you to implement traits.
    /// Dynamic dispatch is made over heap fat ptrs and that means all message objects can be
    /// passed around with heap constructs.
    ///
    /// # Arguments
    /// * `thunk` - User code which will be executed inside the process.
    /// * `msg` - Initial message which will be passed to the thunk.
    /// * `scale` - How many children will be spawn with given `thunk` and `msg` as process body.
    ///
    /// # Examples
    /// ```
    /// use bastion::prelude::*;
    ///
    /// fn main() {
    ///     Bastion::platform();
    ///     Bastion::spawn(|context: BastionContext, _msg: Box<dyn Message>| {
    ///         println!("root supervisor - spawn_at_root - 1");
    ///
    ///         // Let's spawn a child from here
    ///         context.clone().spawn(
    ///               |sub_p: BastionContext, sub_msg: Box<dyn Message>| {
    ///                      receive! { sub_msg,
    ///                         i32 => |msg| { println!("An integer message: {}", msg)},
    ///                         _ => println!("Message not known")
    ///                      }
    ///
    ///                      /// Use blocking hook to commence restart of children
    ///                      /// that has finished their jobs.
    ///                      sub_p.blocking_hook();
    ///               },
    ///               9999_i32, // Initial message which is passed down to child.
    ///               1, // How many children with this body will be spawned
    ///         );
    ///
    ///         // Rebind to the system
    ///         context.blocking_hook();
    ///     }, String::from("A Message"));
    ///
    ///     // Comment out to start the system, so runtime can initialize.
    ///     // Bastion::start()
    /// }
    /// ```
    pub fn spawn<F, M>(self, thunk: F, msg: M, scale: i32) -> Self
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
