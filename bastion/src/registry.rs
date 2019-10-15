use crate::broadcast::{BastionMessage, Sender};
use crate::children::{Child, Children};
use crate::context::{BastionId};
use crate::supervisor::Supervisor;
use chashmap::CHashMap;

pub(super) struct Registry {
	registered: CHashMap<BastionId, Registrant>,
}

struct Registrant {
	sender: Sender,
	ty: RegistrantType,
}

#[derive(Eq, PartialEq)]
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

	pub(super) fn remove_supervisor(&self, supervisor: &Supervisor) {
		let id = supervisor.id();

		self.registered.remove(id);
	}

	pub(super) fn remove_children(&self, children: &Children) {
		let id = children.id();

		self.registered.remove(id);
	}

	pub(super) fn remove_child(&self, child: &Child) {
		let id = child.id();

		self.registered.remove(id);
	}

	pub(super) fn send_supervisor(&self, id: &BastionId, msg: BastionMessage) -> Result<(), BastionMessage> {
		let registrant = if let Some(registrant) = self.registered.get(id) {
			registrant
		} else {
			return Err(msg);
		};

		if !registrant.is_supervisor() {
			return Err(msg);
		}

		registrant.sender.unbounded_send(msg).map_err(|err| err.into_inner())
	}

	pub(super) fn send_children(&self, id: &BastionId, msg: BastionMessage) -> Result<(), BastionMessage> {
		let registrant = if let Some(registrant) = self.registered.get(id) {
			registrant
		} else {
			return Err(msg);
		};

		if !registrant.is_children() {
			return Err(msg);
		}

		registrant.sender.unbounded_send(msg).map_err(|err| err.into_inner())
	}

	pub(super) fn send_child(&self, id: &BastionId, msg: BastionMessage) -> Result<(), BastionMessage> {
		let registrant = if let Some(registrant) = self.registered.get(id) {
			registrant
		} else {
			return Err(msg);
		};

		if !registrant.is_child() {
			return Err(msg);
		}

		registrant.sender.unbounded_send(msg).map_err(|err| err.into_inner())
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

	fn is_supervisor(&self) -> bool {
		self.ty == RegistrantType::Supervisor
	}

	fn is_children(&self) -> bool {
		self.ty == RegistrantType::Children
	}

	fn is_child(&self) -> bool {
		self.ty == RegistrantType::Child
	}
}
