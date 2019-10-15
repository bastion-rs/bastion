use crate::broadcast::{BastionMessage, Broadcast};
use crate::context::BastionId;
use crate::supervisor::Supervisor;
use futures::{pending, poll};
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::prelude::*;
use fxhash::{FxHashMap, FxHashSet};
use runtime::task::JoinHandle;
use std::task::Poll;

pub(super) struct System {
	bcast: Broadcast,
	recver: UnboundedReceiver<Supervisor>,
	supervisors: FxHashMap<BastionId, JoinHandle<Supervisor>>,
	dead: FxHashSet<BastionId>,
}

impl System {
	pub(super) fn start() -> UnboundedSender<Supervisor> {
		let bcast = Broadcast::new();
		let (sender, recver) = mpsc::unbounded();

		let supervisors = FxHashMap::default();
		let dead = FxHashSet::default();

		let system = System {
			bcast,
			recver,
			supervisors,
			dead,
		};

		runtime::spawn(system.run());

		sender
	}

	fn launch_supervisor(&mut self, mut supervisor: Supervisor) {
		supervisor.launch_children();

		runtime::spawn(supervisor.run());
	}

	async fn run(mut self) {
		loop {
			let mut ready = false;

			match poll!(&mut self.bcast.next()) {
				Poll::Ready(Some(msg)) => {
					ready = true;

					match msg {
						// FIXME
						BastionMessage::PoisonPill => unimplemented!(),
						BastionMessage::Dead { id } => {
							self.supervisors.remove(&id);
							self.bcast.remove_child(&id);

							self.dead.insert(id);
						}
						BastionMessage::Faulted { id } => {
							// TODO: add a "faulted" list and poll from it instead of awaiting

							// FIXME: Err if None?
							if let Some(launched) = self.supervisors.remove(&id) {
								let mut supervisor = launched.await;

								supervisor.reset().await;

								// FIXME: set a limit?
								self.launch_supervisor(supervisor);
							}
						}
						BastionMessage::Message(_) => {
							self.bcast.send_children(msg);
						}
					}
				}
				// FIXME
				Poll::Ready(None) => unimplemented!(),
				Poll::Pending => (),
			}

			match poll!(&mut self.recver.next()) {
				Poll::Ready(Some(supervisor)) => {
					ready = true;

					self.launch_supervisor(supervisor);
				}
				// FIXME
				Poll::Ready(None) => unimplemented!(),
				Poll::Pending => (),
			}

			if !ready {
				pending!();
			}
		}
	}
}