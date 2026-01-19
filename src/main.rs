use oxidesk::bootstrap;
use oxidesk::config::Config;
use oxidesk::infrastructure::http::router::build_router;
use oxidesk::infrastructure::persistence::Database;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = Config::from_env()?;

    // Initialize observability
    let _guard = oxidesk::infrastructure::observability::init(&config)?;
    tracing::info!("Observability initialized");
    log::info!("Testing log crate to tracing bridge");
    tracing::info!("Configuration loaded");

    // Initialize database connection
    let db = Database::connect(&config.database_url).await?;
    tracing::info!("Database connection established");

    // Run migrations
    db.run_migrations().await?;
    tracing::info!("Database migrations applied");

    // Initialize admin user
    if let Err(e) = bootstrap::initialize_admin(&db, &config).await {
        tracing::error!("Failed to initialize admin user: {}", e);
        return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()).into());
    }

    // Build application state (and start background services)
    let state = bootstrap::build_app_state(db, &config).await?;

    // Build router
    let app = build_router(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server_port));
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
