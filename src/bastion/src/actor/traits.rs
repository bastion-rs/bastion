use std::fmt::Debug;

use async_trait::async_trait;

use crate::actor::context::Context;
use crate::error::Result;

#[async_trait]
pub trait Actor: 'static {
    async fn on_init(&mut self, _ctx: &mut Context) {}
    async fn on_sync(&mut self, _ctx: &mut Context) {}
    async fn on_stopped(&mut self, _ctx: &mut Context) {}
    async fn on_terminated(&mut self, _ctx: &mut Context) {}
    async fn on_failed(&mut self, _ctx: &mut Context) {}
    async fn on_finished(&mut self, _ctx: &mut Context) {}
    async fn on_deinit(&mut self, _ctx: &mut Context) {}
    async fn handler(&mut self, ctx: &mut Context) -> Result<()>;
}
