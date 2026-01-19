#![allow(ambiguous_glob_reexports)]

// Core hexagonal architecture layers
pub mod application;
pub mod config;
pub mod domain;
pub mod infrastructure;
pub mod shared;

// Re-exports for backward compatibility and convenience
pub use application::services::*;
pub use config::*;
pub use domain::entities::*;
pub use domain::ports::event_bus::EventBus;
pub use domain::ports::*;
pub use infrastructure::http::middleware::error::*;
pub use infrastructure::persistence::Database;
pub use shared::events::{LocalEventBus, SystemEvent};
pub use shared::*;
pub mod bootstrap;
