use crate::bastion::REGISTRY;
use crate::broadcast::{BastionMessage, Sender};
use crate::children::Message;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct BastionID(Uuid);

pub struct BastionContext {
	id: BastionID,
	parent: Sender,
}

impl BastionContext {
	pub(super) fn new(id: Uuid, parent: Sender) -> Self {
		let id = BastionID(id);

		BastionContext { id, parent }
	}

	pub fn id(&self) -> &BastionID {
		&self.id
	}

	pub fn send_msg(&self, id: &BastionID, msg: Box<dyn Message>) -> Result<(), Box<dyn Message>> {
		let msg = BastionMessage::msg(msg);

		REGISTRY.send_child(&id.0, msg).map_err(|msg| msg.into_msg().unwrap())
	}
}
