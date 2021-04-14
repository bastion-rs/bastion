use async_trait::async_trait;

use crate::actor::context::Context;
use crate::errors::BastionResult;

#[async_trait]
pub trait Actor: Sync {
    async fn on_init(&self, _ctx: &mut Context) {}
    async fn on_sync(&self, _ctx: &mut Context) {}
    async fn on_stopped(&self, _ctx: &mut Context) {}
    async fn on_terminated(&self, _ctx: &mut Context) {}
    async fn on_failed(&self, _ctx: &mut Context) {}
    async fn on_finished(&self, _ctx: &mut Context) {}
    async fn on_deinit(&self, _ctx: &mut Context) {}
    async fn handler(&self, ctx: &mut Context) -> BastionResult<()>;
}
