#![allow(ambiguous_glob_reexports)]

// Core hexagonal architecture layers
pub mod domain;
pub mod application;
pub mod infrastructure;
pub mod shared;
pub mod config;

// Re-exports for backward compatibility and convenience
pub use config::*;
pub use domain::entities::*;
pub use domain::ports::*;
pub use application::services::*;
pub use infrastructure::persistence::Database;
pub use infrastructure::http::middleware::error::*;
pub use shared::events::{EventBus, LocalEventBus, SystemEvent};
pub use shared::*;
