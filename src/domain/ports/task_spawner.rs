use futures::future::BoxFuture;

/// Configurable trait for spawning background tasks
/// Allows abstracting the runtime (Tokio) for testing or other environments
pub trait TaskSpawner: Send + Sync {
    /// Spawn a future that returns nothing
    fn spawn(&self, future: BoxFuture<'static, ()>);
}
