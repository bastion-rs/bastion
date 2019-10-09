use crate::supervisor::Supervisor;
use crate::system::System;
use futures::channel::mpsc::UnboundedSender;
use lazy_static::lazy_static;

lazy_static! {
	pub(super) static ref SYSTEM_SENDER: UnboundedSender<Supervisor> = System::start();
}

pub struct Bastion {
	// TODO: ...
}

impl Bastion {
	pub fn supervisor() -> Supervisor {
		Supervisor::new()
	}
}