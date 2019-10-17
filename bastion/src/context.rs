use crate::bastion::REGISTRY;
use crate::broadcast::{BastionMessage, Sender};
use crate::children::Message;
use futures::pending;
use qutex::{Guard, Qutex};
use std::collections::VecDeque;
use uuid::Uuid;

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct BastionId(Uuid);

pub struct BastionContext {
	id: BastionId,
	parent: Sender,
	state: Qutex<ContextState>,
}

pub(super) struct ContextState {
	msgs: VecDeque<Box<dyn Message>>,
}

impl BastionId {
	pub(super) fn new() -> Self {
		let uuid = Uuid::new_v4();

		BastionId(uuid)
	}
}

impl BastionContext {
	pub(super) fn new(id: BastionId, parent: Sender, state: Qutex<ContextState>) -> Self {
		BastionContext { id, parent, state }
	}

	pub fn id(&self) -> &BastionId {
		&self.id
	}

	pub fn send_msg(&self, id: &BastionId, msg: Box<dyn Message>) -> Result<(), Box<dyn Message>> {
		let msg = BastionMessage::msg(msg);

		// TODO: Err(Error)
		REGISTRY.send_child(id, msg).map_err(|msg| msg.into_msg().unwrap())
	}

	// TODO: Err(Error)
	pub async fn recv(&self) -> Result<Box<dyn Message>, ()> {
		loop {
			// TODO: Err(Error)
			let mut state = self.state.clone().lock_async().await.unwrap();

			if let Some(msg) = state.msgs.pop_front() {
				return Ok(msg);
			}

			Guard::unlock(state);

			pending!();
		}
	}

	pub async fn try_recv(&self) -> Option<Box<dyn Message>> {
		// TODO: Err(Error)
		let mut state = self.state.clone().lock_async().await.ok()?;

		state.msgs.pop_front()
	}
}

impl ContextState {
	pub(super) fn new() -> Self {
		let msgs = VecDeque::new();

		ContextState { msgs }
	}

	pub(super) fn push_msg(&mut self, msg: Box<dyn Message>) {
		self.msgs.push_back(msg)
	}
}
