use crate::bastion::REGISTRY;
use crate::broadcast::{BastionMessage, Sender};
use crate::children::Message;
use uuid::Uuid;

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct BastionId(Uuid);

pub struct BastionContext {
	id: BastionId,
	parent: Sender,
}

impl BastionId {
	pub(super) fn new() -> Self {
		let uuid = Uuid::new_v4();

		BastionId(uuid)
	}
}

impl BastionContext {
	pub(super) fn new(id: BastionId, parent: Sender) -> Self {
		BastionContext { id, parent }
	}

	pub fn id(&self) -> &BastionId {
		&self.id
	}

	pub fn send_msg(&self, id: &BastionId, msg: Box<dyn Message>) -> Result<(), Box<dyn Message>> {
		let msg = BastionMessage::msg(msg);

		REGISTRY.send_child(id, msg).map_err(|msg| msg.into_msg().unwrap())
	}
}
