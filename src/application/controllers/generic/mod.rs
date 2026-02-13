/// Framework-Agnostic Controllers
///
/// Controllers in this module are generic over HttpRuntime, allowing them
/// to work with both Axum (traditional servers) and worker-rs (Cloudflare Workers).

pub mod auth_controller;

pub use auth_controller::GenericAuthController;
