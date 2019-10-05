//!
//! Supervisor related code. Including their management, creation, and destruction.

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
    /// Supervisor's system name
    pub sys: String,
    /// Supervisor's name
    pub name: String,
    /// Supervisor's unique identifier
    pub res: String,
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

///
/// Supervisor definition to keep track of supervisor information. e.g:
/// * context
/// * children
/// * strategy
/// * supervisor identifier
#[derive(Default, Clone, Debug)]
pub struct Supervisor {
    /// Supervisor's URN scheme
    pub urn: SupervisorURN,
    /// Supervisor's context
    pub(crate) ctx: BastionContext,
    /// Supervisor's strategy
    pub(crate) strategy: SupervisionStrategy,
}

///
/// Builder pattern for supervisors.
impl Supervisor {
    ///
    /// Assign properties of the supervisor
    ///
    /// # Arguments
    /// * `name` - name of the supervisor
    /// * `system` - system name for the supervisor. Can be used for grouping dependent supervisors.
    ///
    /// # Example
    /// ```rust
    ///# use bastion::prelude::*;
    ///# let name = "supervisor-name";
    ///# let system = "system-name";
    /// Supervisor::default().props(name.into(), system.into());
    /// ```
    pub fn props(mut self, name: String, system: String) -> Self {
        let mut urn = SupervisorURN::default();
        urn.name = name;
        self.urn = urn;
        self.urn.sys = system;
        self
    }

    ///
    /// Assigns strategy for this supervisor
    ///
    /// # Arguments
    /// * `strategy` - An [SupervisionStrategy] for supervisor to define how to operate on failures.
    ///
    /// # Example
    /// ```rust
    ///# use bastion::prelude::*;
    ///# let name = "supervisor-name";
    ///# let system = "system-name";
    /// Supervisor::default().props(name.into(), system.into())
    ///    .strategy(SupervisionStrategy::RestForOne);
    /// ```
    pub fn strategy(mut self, strategy: SupervisionStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    ///
    /// [Supervisor] level spawn function for child generation from the parent context.
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
    ///# use bastion::prelude::*;
    ///# use std::{fs, thread};
    ///#
    ///# fn main() {
    ///#     Bastion::platform();
    ///#
    ///#     let message = "Supervision Message".to_string();
    ///#
    ///#     /// Name of the supervisor, and system of the new supervisor
    ///#     /// By default if you don't specify Supervisors use "One for One".
    ///#     /// Let's look at "One for One".
    ///Bastion::supervisor("file-reader", "remote-fs")
    ///    .strategy(SupervisionStrategy::OneForOne)
    ///    .children(
    ///        |p: BastionContext, _msg| {
    ///            println!("File below doesn't exist so it will panic.");
    ///            fs::read_to_string("cacophony").unwrap();
    ///
    ///            /// Hook to rebind to the system.
    ///            p.hook();
    ///        },
    ///        message, // Message for all redundant children
    ///        1_i32,   // Redundancy level
    ///    );
    ///# }
    /// ```
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

    ///
    /// Launch completes the builder pattern.
    /// It is the main finalizer that sets necessary arguments prepares
    /// channels and registers supervisor to the runtime.
    ///
    /// Runtime can't be notified without calling [launch](struct.Supervisor.html#method.launch).
    ///
    /// # Examples
    /// ```
    ///# use bastion::prelude::*;
    ///# use std::{fs, thread};
    ///#
    ///# fn main() {
    ///#     Bastion::platform();
    ///#
    ///#     let message = "Supervision Message".to_string();
    ///#
    ///#     /// Name of the supervisor, and system of the new supervisor
    ///#     /// By default if you don't specify Supervisors use "One for One".
    ///#     /// Let's look at "One for One".
    ///Bastion::supervisor("file-reader", "remote-fs")
    ///    .strategy(SupervisionStrategy::OneForOne)
    ///    .children(
    ///        |p: BastionContext, _msg| {
    ///            println!("File below doesn't exist so it will panic.");
    ///            fs::read_to_string("cacophony").unwrap();
    ///
    ///            /// Hook to rebind to the system.
    ///            p.hook();
    ///        },
    ///        message, // Message for all redundant children
    ///        1_i32,   // Redundancy level
    ///    )
    ///    .launch(); // Launch finalizes supervisor build and registers to definition.
    ///# }
    /// ```
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
