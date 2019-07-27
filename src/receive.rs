use crate::child::Message;

pub struct Receive<T>(pub Option<T>);

impl<T: 'static + Clone> From<Box<dyn Message>> for Receive<T> {
    fn from(message: Box<dyn Message>) -> Self {
        Receive(message.as_any().downcast_ref::<T>().cloned())
    }
}
