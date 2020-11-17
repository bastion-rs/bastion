use std::sync::Arc;

use crate::actor::context::Context;
use crate::mailbox::traits::TypedMessage;

#[derive(Default, Clone)]
pub struct Callbacks<T>
where
    T: TypedMessage,
{
    on_init: Option<Arc<dyn Fn(&mut Context<T>) + Send + Sync>>,
    on_sync: Option<Arc<dyn Fn(&mut Context<T>) + Send + Sync>>,
    on_stopped: Option<Arc<dyn Fn(&mut Context<T>) + Send + Sync>>,
    on_terminated: Option<Arc<dyn Fn(&mut Context<T>) + Send + Sync>>,
    on_failed: Option<Arc<dyn Fn(&mut Context<T>) + Send + Sync>>,
    on_finished: Option<Arc<dyn Fn(&mut Context<T>) + Send + Sync>>,
    on_deinit: Option<Arc<dyn Fn(&mut Context<T>) + Send + Sync>>,
}

impl<T> Callbacks<T>
where
    T: TypedMessage,
{
    pub fn new() -> Self {
        Callbacks::default()
    }
}
