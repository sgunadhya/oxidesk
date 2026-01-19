use async_trait::async_trait;
use std::time::Duration;

#[async_trait]
pub trait TimeService: Send + Sync {
    async fn sleep(&self, duration: Duration);
}
