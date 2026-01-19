use crate::domain::events::SystemEvent;
use crate::infrastructure::http::middleware::error::ApiResult;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

#[async_trait]
pub trait EventBus: Send + Sync {
    /// Publish an event to all subscribers
    fn publish(&self, event: SystemEvent) -> ApiResult<()>;

    /// Subscribe to events
    /// Returns a stream of events, abstracting away underlying transport errors
    fn subscribe(&self) -> Pin<Box<dyn Stream<Item = Result<SystemEvent, String>> + Send>>;
}
