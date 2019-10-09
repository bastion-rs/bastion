use crate::broadcast::Sender;
use uuid::Uuid;

pub struct BastionContext {
	id: Uuid,
	parent: Sender,
}

impl BastionContext {
	pub(super) fn new(id: Uuid, parent: Sender) -> Self {
		BastionContext { id, parent }
	}

	pub fn id(&self) -> &Uuid {
		&self.id
	}
}
