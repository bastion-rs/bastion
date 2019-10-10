use crate::broadcast::Sender;
use crate::children::{Child, Children};
use crate::supervisor::Supervisor;
use chashmap::CHashMap;
use uuid::Uuid;

pub(super) struct Registry {
	registered: CHashMap<Uuid, Registrant>,
}

struct Registrant {
	sender: Sender,
	ty: RegistrantType,
}

enum RegistrantType {
	Supervisor,
	Children,
	Child,
}

impl Registry {
	pub(super) fn new() -> Self {
		// FIXME: with_capacity?
		let registered = CHashMap::new();

		Registry { registered }
	}

	pub(super) fn add_supervisor(&self, supervisor: &Supervisor) {
		let id = supervisor.id().clone();
		let sender = supervisor.sender().clone();

		let registrant = Registrant::supervisor(sender);

		self.registered.insert(id, registrant);
	}

	pub(super) fn add_children(&self, children: &Children) {
		let id = children.id().clone();
		let sender = children.sender().clone();

		let registrant = Registrant::children(sender);

		self.registered.insert(id, registrant);
	}

	pub(super) fn add_child(&self, child: &Child) {
		let id = child.id().clone();
		let sender = child.sender().clone();

		let registrant = Registrant::child(sender);

		self.registered.insert(id, registrant);
	}
}

impl Registrant {
	fn supervisor(sender: Sender) -> Self {
		let ty = RegistrantType::Supervisor;

		Registrant { sender, ty }
	}

	fn children(sender: Sender) -> Self {
		let ty = RegistrantType::Children;

		Registrant { sender, ty }
	}

	fn child(sender: Sender) -> Self {
		let ty = RegistrantType::Child;

		Registrant { sender, ty }
	}
}
