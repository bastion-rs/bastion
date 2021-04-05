use crate::{
    child_ref::SendError,
    context::BastionId,
    dispatcher::RecipientTarget,
    message::Message,
    message::Msg,
    path::BastionPath,
    prelude::{RefAddr, SignedMessage},
    system::SYSTEM,
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Recipient {
    target: RecipientTarget,
    sign: RefAddr,
    path: Arc<BastionPath>,
}

impl Recipient {
    pub fn named(name: &str) -> Self {
        let path = Arc::new(BastionPath::recipient(BastionId::new()));
        Self {
            sign: RefAddr::new(Arc::clone(&path), SYSTEM.sender().clone()),
            path,
            target: RecipientTarget::named(name),
        }
    }

    pub fn tell<M: Message>(&self, message: M) -> Result<(), SendError> {
        self.send(Msg::tell(message)).map_err(Into::into)
    }

    pub fn addr(&self) -> RefAddr {
        self.sign.clone()
    }

    fn send(&self, msg: Msg) -> Result<(), SendError> {
        todo!();
        // let signed_message = SignedMessage {
        //     msg,
        //     sign: self.sign.clone(),
        // };

        // let global_dispatcher = SYSTEM.dispatcher();
        // global_dispatcher.send_to_recipient(self.target, signed_message)
    }
}
