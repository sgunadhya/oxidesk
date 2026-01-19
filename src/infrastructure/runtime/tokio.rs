use crate::domain::ports::task_spawner::TaskSpawner;
use crate::domain::ports::time_service::TimeService;
use async_trait::async_trait;
use futures::future::BoxFuture;
use std::time::Duration;

#[derive(Clone)]
pub struct TokioTaskSpawner;

impl TokioTaskSpawner {
    pub fn new() -> Self {
        Self
    }
}

impl TaskSpawner for TokioTaskSpawner {
    fn spawn(&self, future: BoxFuture<'static, ()>) {
        tokio::spawn(future);
    }
}

#[derive(Clone)]
pub struct TokioTimeService;

impl TokioTimeService {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TimeService for TokioTimeService {
    async fn sleep(&self, duration: Duration) {
        tokio::time::sleep(duration).await;
    }
}
