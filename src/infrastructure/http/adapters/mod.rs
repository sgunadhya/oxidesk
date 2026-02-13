/// HTTP Runtime Adapters
///
/// This module contains concrete implementations of the HttpRuntime traits
/// for different frameworks and platforms.

#[cfg(feature = "axum-runtime")]
pub mod axum_adapter;

#[cfg(feature = "worker-runtime")]
pub mod worker_adapter;
