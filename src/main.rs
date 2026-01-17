// Modules are imported from the library crate

use oxidesk::domain::ports::agent_repository::AgentRepository;
use oxidesk::domain::ports::user_repository::UserRepository;
use oxidesk::{
    api::{
        self,
        middleware::{ApiError, AppState},
    }, // Explicitly fixing imports mapping
    config::Config,
    database::Database,
    models::*,
    services::{
        connection_manager::{ConnectionManager, InMemoryConnectionManager},
        *,
    },
    web,
    EventBus,
};
// Re-import initialize_admin for main.rs usage if it was public in lib?
// initialize_admin was defined in main.rs (line 305). Wait.
// If initialize_admin function is at the bottom of main.rs, it uses `Database`.
// `Database` is now `oxidesk::database::Database`.

use axum::{
    extract::State,
    routing::{delete, get, patch, post, put},
    Router,
};
use std::sync::Arc;
use tokio_stream::StreamExt;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "oxidesk=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Install default drivers for sqlx::Any
    sqlx::any::install_default_drivers();

    tracing::info!("Starting Oxidesk User Management System");

    // Load configuration
    let config = Config::from_env().map_err(|e| {
        tracing::error!("Configuration error: {}", e);
        e
    })?;

    tracing::info!("Database URL: {}", config.database_url);

    // Connect to database
    let db = Database::connect(&config.database_url).await.map_err(|e| {
        tracing::error!("Failed to connect to database: {}", e);
        e
    })?;

    tracing::info!("Connected to database successfully");

    let attachment_storage_path =
        std::env::var("ATTACHMENT_STORAGE_PATH").unwrap_or_else(|_| "./attachments".to_string());
    std::fs::create_dir_all(&attachment_storage_path)?;

    // Run migrations
    db.run_migrations().await.map_err(|e| {
        tracing::error!("Failed to run migrations: {}", e);
        e
    })?;

    tracing::info!("Database migrations completed");

    // Initialize system with admin user
    initialize_admin(&db, &config).await.map_err(|e| {
        tracing::error!("Failed to initialize admin user: {}", e);
        e
    })?;

    // Initialize event bus for automation rules
    let event_bus = std::sync::Arc::new(oxidesk::LocalEventBus::new(100));
    tracing::info!("Event bus initialized with capacity 100");

    // Initialize delivery service with mock provider
    use oxidesk::domain::ports::message_repository::MessageRepository;
    let delivery_provider = std::sync::Arc::new(oxidesk::services::MockDeliveryProvider::new());
    let delivery_service = oxidesk::services::DeliveryService::new(
        Arc::new(db.clone()) as Arc<dyn MessageRepository>,
        delivery_provider,
    );
    tracing::info!("Delivery service initialized with mock provider");

    // Initialize notification service
    let notification_service = oxidesk::NotificationService::new();
    tracing::info!("Notification service initialized (stub)");

    // Initialize availability service
    let availability_service = oxidesk::AvailabilityService::new(db.clone(), event_bus.clone());
    tracing::info!("Availability service initialized");

    // Initialize SLA service
    let sla_service = oxidesk::SlaService::new(db.clone(), event_bus.clone());
    tracing::info!("SLA service initialized");

    // Initialize automation service
    let automation_service = std::sync::Arc::new(oxidesk::AutomationService::new(
        std::sync::Arc::new(db.clone()),
        AutomationConfig::default(),
    ));
    // Initialize webhook service
    let webhook_repo = oxidesk::domain::ports::webhook_repository::WebhookRepository::new(db.clone());
    let webhook_service = oxidesk::WebhookService::new(webhook_repo);

    // Initialize Tag Repository
    let tag_repo = oxidesk::domain::ports::tag_repository::TagRepository::new(db.clone());

    // Initialize Conversation Tag Service
    use oxidesk::domain::ports::conversation_tag_repository::ConversationTagRepository;
    let conversation_tag_service = ConversationTagService::new(
        Arc::new(db.clone()) as Arc<dyn ConversationTagRepository>,
        tag_repo.clone(),
        Arc::new(db.clone()) as Arc<dyn ConversationRepository>,
        event_bus.clone(),
    );
    tracing::info!("Conversation tag service initialized");

    // Initialize Conversation Priority Service
    use oxidesk::domain::ports::conversation_repository::ConversationRepository;
    let conversation_priority_service = oxidesk::ConversationPriorityService::new(
        Arc::new(db.clone()) as Arc<dyn ConversationRepository>,
        Some(event_bus.clone()),
    );
    tracing::info!("Conversation priority service initialized");

    use oxidesk::domain::ports::agent_repository::AgentRepository;
    use oxidesk::domain::ports::inbox_repository::InboxRepository;
    use oxidesk::domain::ports::role_repository::RoleRepository;
    use oxidesk::domain::ports::user_repository::UserRepository;

    let inbox_service = InboxService::new(Arc::new(db.clone()) as Arc<dyn InboxRepository>);
    let tag_service = TagService::new(tag_repo.clone());
    let role_service = RoleService::new(Arc::new(db.clone()) as Arc<dyn RoleRepository>);
    tracing::info!("Automation service initialized");
    let connection_manager: Arc<dyn ConnectionManager> = Arc::new(InMemoryConnectionManager::new());
    tracing::info!("Connection manager initialized");

    // Initialize rate limiter
    let rate_limiter = oxidesk::services::AuthRateLimiter::new();
    tracing::info!("Rate limiter initialized (5 attempts per 15 minutes)");

    // Initialize TaskQueue
    let task_queue = std::sync::Arc::new(oxidesk::services::SqliteTaskQueue::new(db.clone()));

    // Enqueue initial maintenance jobs
    let q_init = task_queue.clone();
    tokio::spawn(async move {
        if let Err(e) = q_init
            .enqueue("cleanup_sessions", serde_json::Value::Null, 3)
            .await
        {
            tracing::error!("Failed to enqueue initial cleanup_sessions: {}", e);
        }
        if let Err(e) = q_init
            .enqueue("cleanup_rate_limiter", serde_json::Value::Null, 3)
            .await
        {
            tracing::error!("Failed to enqueue initial cleanup_rate_limiter: {}", e);
        }
        if let Err(e) = q_init
            .enqueue("cleanup_oidc_states", serde_json::Value::Null, 3)
            .await
        {
            tracing::error!("Failed to enqueue initial cleanup_oidc_states: {}", e);
        }
        if let Err(e) = q_init
            .enqueue("check_availability", serde_json::Value::Null, 3)
            .await
        {
            tracing::error!("Failed to enqueue initial check_availability: {}", e);
        }
        if let Err(e) = q_init
            .enqueue("check_sla_breaches", serde_json::Value::Null, 3)
            .await
        {
            tracing::error!("Failed to enqueue initial check_sla_breaches: {}", e);
        }
    });

    // Initialize Session Service
    let session_service = oxidesk::services::SessionService::new(std::sync::Arc::new(db.clone()));
    tracing::info!("Session service initialized");

    // Start JobProcessor
    let job_processor = oxidesk::services::JobProcessor::new(
        task_queue.clone(),
        db.clone(),
        rate_limiter.clone(),
        availability_service.clone(),
        sla_service.clone(),
        session_service.clone(),
    );
    tokio::spawn(async move {
        job_processor.start().await;
    });

    // Initialize Agent Service
    let agent_service = oxidesk::services::AgentService::new(
        std::sync::Arc::new(db.clone()) as Arc<dyn AgentRepository>,
        std::sync::Arc::new(db.clone()) as Arc<dyn UserRepository>,
        std::sync::Arc::new(db.clone()) as Arc<dyn RoleRepository>,
        session_service.clone(),
    );
    tracing::info!("Agent service initialized");

    // Initialize User Service
    let user_service = oxidesk::services::UserService::new(std::sync::Arc::new(db.clone()));
    tracing::info!("User service initialized");

    // Initialize Contact Service
    let contact_service = oxidesk::services::ContactService::new(
        std::sync::Arc::new(db.clone()),
        std::sync::Arc::new(db.clone()),
    );
    tracing::info!("Contact service initialized");

    // Initialize Repositories
    let email_repo: std::sync::Arc<dyn oxidesk::domain::ports::email_repository::EmailRepository> =
        std::sync::Arc::new(db.clone());
    let attachment_repo: std::sync::Arc<
        dyn oxidesk::domain::ports::attachment_repository::AttachmentRepository,
    > = std::sync::Arc::new(db.clone());
    let conversation_repo: std::sync::Arc<
        dyn oxidesk::domain::ports::conversation_repository::ConversationRepository,
    > = std::sync::Arc::new(db.clone());
    let message_repo: std::sync::Arc<
        dyn oxidesk::domain::ports::message_repository::MessageRepository,
    > = std::sync::Arc::new(db.clone());
    let user_repo: std::sync::Arc<dyn oxidesk::domain::ports::user_repository::UserRepository> =
        std::sync::Arc::new(db.clone());
    let contact_repo: std::sync::Arc<
        dyn oxidesk::domain::ports::contact_repository::ContactRepository,
    > = std::sync::Arc::new(db.clone());
    let team_repo: std::sync::Arc<dyn oxidesk::domain::ports::team_repository::TeamRepository> =
        std::sync::Arc::new(db.clone());
    let team_service = oxidesk::services::TeamService::new(team_repo.clone());

    // Initialize OIDC service
    let oidc_repository = oxidesk::domain::ports::oidc_repository::OidcRepository::new(db.clone());
    let oidc_service = oxidesk::services::OidcService::new(oidc_repository);
    tracing::info!("OIDC service initialized");

    // Initialize Services (wrapping repositories)
    let email_service = oxidesk::services::EmailService::new(email_repo.clone());
    let attachment_service = oxidesk::services::AttachmentService::new(
        attachment_repo.clone(),
        std::path::PathBuf::from(&attachment_storage_path),
    );

    let conversation_service = oxidesk::services::ConversationService::new(
        conversation_repo.clone(),
        user_repo.clone(),
        contact_repo.clone(),
        team_repo.clone(),
    );

    let message_service = oxidesk::services::MessageService::with_all_services(
        message_repo.clone(),
        conversation_repo.clone(),
        delivery_service.clone(),
        event_bus.clone(),
        connection_manager.clone(),
    );

    // Initialize MacroService
    let macro_repo = oxidesk::domain::ports::macro_repository::MacroRepository::new(db.clone());
    let macro_service = oxidesk::services::MacroService::new(macro_repo);

    // Initialize AuthService
    let auth_service = oxidesk::services::AuthService::new(
        Arc::new(db.clone()) as Arc<dyn UserRepository>,
        Arc::new(db.clone()) as Arc<dyn AgentRepository>,
        Arc::new(db.clone()) as Arc<dyn RoleRepository>,
        session_service.clone(),
    );

    // Initialize PasswordResetService
    let password_reset_repo = oxidesk::domain::ports::password_reset_repository::PasswordResetRepository::new(db.clone());
    let password_reset_service = oxidesk::services::PasswordResetService::new(
        password_reset_repo,
        Arc::new(db.clone()) as Arc<dyn UserRepository>,
    );
    tracing::info!("Password reset service initialized");

    // Create application state
    let state = AppState {
        db: db.clone(),
        session_duration_hours: config.session_duration_hours,
        event_bus: event_bus.clone(),
        delivery_service: delivery_service.clone(),
        notification_service: notification_service.clone(),
        availability_service: availability_service.clone(),
        sla_service: sla_service.clone(),
        automation_service: automation_service.clone(),
        conversation_tag_service: conversation_tag_service.clone(),
        connection_manager,
        rate_limiter,
        webhook_service: webhook_service.clone(),
        tag_service: tag_service.clone(),
        agent_service: agent_service.clone(),
        user_service: user_service.clone(),
        contact_service: contact_service.clone(),
        session_service: session_service.clone(),
        email_service,
        attachment_service,
        conversation_service,
        message_service,
        oidc_service,
        macro_service,
        role_service,
        inbox_service,
        auth_service,
        password_reset_service,
        team_service,
        conversation_priority_service,
    };

    // Start automation listener background task
    let automation_event_bus = event_bus.clone();
    let automation_sla_service = sla_service.clone();
    let automation_db = db.clone();
    tokio::spawn(async move {
        // Initialize automation service inside the task
        let automation_rule_service = std::sync::Arc::new(oxidesk::AutomationService::new(
            std::sync::Arc::new(automation_db.clone()),
            oxidesk::services::automation_service::AutomationConfig::default(),
        ));
        tracing::info!("Automation service initialized in background task");

        let mut receiver = automation_event_bus.subscribe();
        tracing::info!("Automation listener started");

        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(event) => {
                    tracing::debug!("Automation listener received event: {:?}", event);

                    // Process automation rules based on event
                    match event {
                        oxidesk::SystemEvent::ConversationCreated {
                            conversation_id,
                            inbox_id,
                            contact_id,
                            status,
                            timestamp,
                        } => {
                            tracing::info!(
                                "Automation: Conversation {} created in inbox {} by contact {} with status {:?} at {}",
                                conversation_id,
                                inbox_id,
                                contact_id,
                                status,
                                timestamp
                            );

                            // Trigger automation rules for conversation creation
                            if let Ok(Some(conversation)) =
                                automation_db.get_conversation_by_id(&conversation_id).await
                            {
                                if let Err(e) = automation_rule_service
                                    .handle_conversation_event(
                                        "conversation.created",
                                        &conversation,
                                        "system",
                                    )
                                    .await
                                {
                                    tracing::error!("Failed to execute automation rules for conversation creation: {}", e);
                                }
                            }
                        }
                        oxidesk::SystemEvent::ConversationStatusChanged {
                            conversation_id,
                            old_status,
                            new_status,
                            agent_id,
                            timestamp,
                        } => {
                            tracing::info!(
                                "Automation: Conversation {} status changed from {:?} to {:?} by agent {:?} at {}",
                                conversation_id,
                                old_status,
                                new_status,
                                agent_id,
                                timestamp
                            );

                            // Handle resolution SLA
                            if new_status == oxidesk::ConversationStatus::Resolved {
                                if let Err(e) = automation_sla_service
                                    .handle_conversation_resolved(&conversation_id, &timestamp)
                                    .await
                                {
                                    tracing::error!("Failed to handle resolution SLA: {}", e);
                                }
                            }

                            // Trigger automation rules for status change
                            if let Ok(Some(conversation)) =
                                automation_db.get_conversation_by_id(&conversation_id).await
                            {
                                let executed_by = agent_id.as_deref().unwrap_or("system");
                                if let Err(e) = automation_rule_service
                                    .handle_conversation_event(
                                        "conversation.status_changed",
                                        &conversation,
                                        executed_by,
                                    )
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to execute automation rules for status change: {}",
                                        e
                                    );
                                }
                            }
                        }
                        oxidesk::SystemEvent::MessageReceived {
                            message_id,
                            conversation_id,
                            contact_id,
                            timestamp,
                        } => {
                            tracing::info!(
                                "Automation: Message {} received in conversation {} from contact {} at {}",
                                message_id,
                                conversation_id,
                                contact_id,
                                timestamp
                            );

                            // Handle next response SLA
                            if let Err(e) = automation_sla_service
                                .handle_contact_message(&conversation_id, &contact_id, &timestamp)
                                .await
                            {
                                tracing::error!("Failed to handle next response SLA: {}", e);
                            }

                            // Trigger automation rules for incoming messages
                            if let Ok(Some(conversation)) =
                                automation_db.get_conversation_by_id(&conversation_id).await
                            {
                                if let Err(e) = automation_rule_service
                                    .handle_conversation_event(
                                        "conversation.message_received",
                                        &conversation,
                                        "system",
                                    )
                                    .await
                                {
                                    tracing::error!("Failed to execute automation rules for message received: {}", e);
                                }
                            }
                        }
                        oxidesk::SystemEvent::MessageSent {
                            message_id,
                            conversation_id,
                            agent_id,
                            timestamp,
                        } => {
                            tracing::info!(
                                "Automation: Message {} sent in conversation {} by agent {} at {}",
                                message_id,
                                conversation_id,
                                agent_id,
                                timestamp
                            );

                            // Handle first response SLA
                            if let Err(e) = automation_sla_service
                                .handle_agent_message(&conversation_id, &agent_id, &timestamp)
                                .await
                            {
                                tracing::error!("Failed to handle first response SLA: {}", e);
                            }

                            // Trigger automation rules for sent messages
                            if let Ok(Some(conversation)) =
                                automation_db.get_conversation_by_id(&conversation_id).await
                            {
                                if let Err(e) = automation_rule_service
                                    .handle_conversation_event(
                                        "conversation.message_sent",
                                        &conversation,
                                        &agent_id,
                                    )
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to execute automation rules for message sent: {}",
                                        e
                                    );
                                }
                            }
                        }
                        oxidesk::SystemEvent::MessageFailed {
                            message_id,
                            conversation_id,
                            retry_count,
                            timestamp,
                        } => {
                            tracing::warn!(
                                "Automation: Message {} failed in conversation {} (retry {}) at {}",
                                message_id,
                                conversation_id,
                                retry_count,
                                timestamp
                            );

                            // Trigger automation rules for failed messages
                            if let Ok(Some(conversation)) =
                                automation_db.get_conversation_by_id(&conversation_id).await
                            {
                                if let Err(e) = automation_rule_service
                                    .handle_conversation_event(
                                        "conversation.message_failed",
                                        &conversation,
                                        "system",
                                    )
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to execute automation rules for message failed: {}",
                                        e
                                    );
                                }
                            }
                        }
                        oxidesk::SystemEvent::ConversationAssigned {
                            conversation_id,
                            assigned_user_id,
                            assigned_team_id,
                            assigned_by,
                            timestamp,
                        } => {
                            tracing::info!(
                                "Automation: Conversation {} assigned (user: {:?}, team: {:?}) by {} at {}",
                                conversation_id,
                                assigned_user_id,
                                assigned_team_id,
                                assigned_by,
                                timestamp
                            );

                            // Auto-apply SLA if assigned to a team with a default SLA policy
                            if let Some(team_id) = &assigned_team_id {
                                // Check if conversation already has an applied SLA
                                match automation_sla_service
                                    .get_applied_sla_by_conversation(&conversation_id)
                                    .await
                                {
                                    Ok(None) => {
                                        // No existing SLA, check if team has a default policy
                                        if let Ok(Some(team)) =
                                            automation_db.get_team_by_id(team_id).await
                                        {
                                            if let Some(policy_id) = team.sla_policy_id {
                                                tracing::info!(
                                                    "Auto-applying SLA policy {} to conversation {} (assigned to team {})",
                                                    policy_id,
                                                    conversation_id,
                                                    team_id
                                                );

                                                match automation_sla_service
                                                    .apply_sla(
                                                        &conversation_id,
                                                        &policy_id,
                                                        &timestamp,
                                                    )
                                                    .await
                                                {
                                                    Ok(_) => {
                                                        tracing::info!(
                                                            "Successfully auto-applied SLA policy {} to conversation {}",
                                                            policy_id,
                                                            conversation_id
                                                        );
                                                    }
                                                    Err(e) => {
                                                        tracing::error!(
                                                            "Failed to auto-apply SLA policy {} to conversation {}: {}",
                                                            policy_id,
                                                            conversation_id,
                                                            e
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Ok(Some(_)) => {
                                        tracing::debug!(
                                            "Conversation {} already has an applied SLA, skipping auto-application",
                                            conversation_id
                                        );
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            "Failed to check existing SLA for conversation {}: {}",
                                            conversation_id,
                                            e
                                        );
                                    }
                                }
                            }

                            // Trigger automation rules for assignment change
                            if let Ok(Some(conversation)) =
                                automation_db.get_conversation_by_id(&conversation_id).await
                            {
                                if let Err(e) = automation_rule_service
                                    .handle_conversation_event(
                                        "conversation.assignment_changed",
                                        &conversation,
                                        &assigned_by,
                                    )
                                    .await
                                {
                                    tracing::error!("Failed to execute automation rules for assignment change: {}", e);
                                }
                            }
                            // TODO: Feature 012 will add webhook triggering
                        }
                        oxidesk::SystemEvent::ConversationUnassigned {
                            conversation_id,
                            previous_assigned_user_id,
                            previous_assigned_team_id,
                            unassigned_by,
                            timestamp,
                        } => {
                            tracing::info!(
                                "Automation: Conversation {} unassigned (was user: {:?}, team: {:?}) by {} at {}",
                                conversation_id,
                                previous_assigned_user_id,
                                previous_assigned_team_id,
                                unassigned_by,
                                timestamp
                            );

                            // Trigger automation rules for unassignment
                            if let Ok(Some(conversation)) =
                                automation_db.get_conversation_by_id(&conversation_id).await
                            {
                                if let Err(e) = automation_rule_service
                                    .handle_conversation_event(
                                        "conversation.unassigned",
                                        &conversation,
                                        &unassigned_by,
                                    )
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to execute automation rules for unassignment: {}",
                                        e
                                    );
                                }
                            }
                            // TODO: Feature 012 will add webhook triggering
                        }
                        oxidesk::SystemEvent::ConversationTagsChanged {
                            conversation_id,
                            previous_tags,
                            new_tags,
                            changed_by,
                            timestamp,
                        } => {
                            tracing::info!(
                                "Automation: Conversation {} tags changed by {} at {} (was: {:?}, now: {:?})",
                                conversation_id,
                                changed_by,
                                timestamp,
                                previous_tags,
                                new_tags
                            );

                            // Trigger automation rules for tags change
                            if let Ok(Some(conversation)) =
                                automation_db.get_conversation_by_id(&conversation_id).await
                            {
                                if let Err(e) = automation_rule_service
                                    .handle_conversation_event(
                                        "conversation.tags_changed",
                                        &conversation,
                                        &changed_by,
                                    )
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to execute automation rules for tags change: {}",
                                        e
                                    );
                                }
                            }
                            // TODO: Feature 012 will add webhook triggering
                        }
                        oxidesk::SystemEvent::AgentAvailabilityChanged {
                            agent_id,
                            old_status,
                            new_status,
                            timestamp,
                            reason,
                        } => {
                            tracing::info!(
                                "Automation: Agent {} availability changed from {} to {} ({}) at {}",
                                agent_id,
                                old_status,
                                new_status,
                                reason,
                                timestamp
                            );
                            // TODO: Feature 009 will add automation rule evaluation
                            // TODO: Feature 012 will add webhook triggering
                        }
                        oxidesk::SystemEvent::AgentLoggedIn {
                            agent_id,
                            user_id,
                            timestamp,
                        } => {
                            tracing::info!(
                                "Automation: Agent {} (user {}) logged in at {}",
                                agent_id,
                                user_id,
                                timestamp
                            );
                        }
                        oxidesk::SystemEvent::AgentLoggedOut {
                            agent_id,
                            user_id,
                            timestamp,
                        } => {
                            tracing::info!(
                                "Automation: Agent {} (user {}) logged out at {}",
                                agent_id,
                                user_id,
                                timestamp
                            );
                        }
                        oxidesk::SystemEvent::SlaBreached {
                            event_id,
                            applied_sla_id,
                            conversation_id,
                            event_type,
                            deadline_at,
                            breached_at,
                            timestamp,
                        } => {
                            tracing::warn!(
                                "Automation: SLA breached for conversation {} - event type: {} (event: {}, applied_sla: {}) deadline: {} breached: {} at {}",
                                conversation_id,
                                event_type,
                                event_id,
                                applied_sla_id,
                                deadline_at,
                                breached_at,
                                timestamp
                            );

                            // Trigger automation rules for SLA breach
                            if let Ok(Some(conversation)) =
                                automation_db.get_conversation_by_id(&conversation_id).await
                            {
                                if let Err(e) = automation_rule_service
                                    .handle_conversation_event(
                                        "conversation.sla_breached",
                                        &conversation,
                                        "system",
                                    )
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to execute automation rules for SLA breach: {}",
                                        e
                                    );
                                }
                            }
                            // TODO: Feature 010 will add notification sending
                            // TODO: Feature 012 will add webhook triggering
                        }
                        oxidesk::SystemEvent::ConversationPriorityChanged {
                            conversation_id,
                            previous_priority,
                            new_priority,
                            updated_by,
                            timestamp,
                        } => {
                            tracing::info!(
                                "Automation: Conversation {} priority changed from {:?} to {:?} by {} at {}",
                                conversation_id,
                                previous_priority,
                                new_priority,
                                updated_by,
                                timestamp
                            );

                            // Trigger automation rules for priority change
                            if let Ok(Some(conversation)) =
                                automation_db.get_conversation_by_id(&conversation_id).await
                            {
                                if let Err(e) = automation_rule_service
                                    .handle_conversation_event(
                                        "conversation.priority_changed",
                                        &conversation,
                                        &updated_by,
                                    )
                                    .await
                                {
                                    tracing::error!("Failed to execute automation rules for priority change: {}", e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Automation listener error: {}", e);
                    // Sleep briefly before retrying - Not needed for stream
                }
            }
        }
    });

    // Start webhook worker background task
    let webhook_db = db.clone();
    let webhook_event_bus = event_bus.clone();
    // Pass task_queue to WebhookWorker
    let webhook_worker =
        oxidesk::WebhookWorker::new(webhook_db, webhook_event_bus, task_queue.clone());
    webhook_worker.start();
    tracing::info!("Webhook worker started");

    // Build protected routes (require authentication)
    let protected = Router::new()
        .route("/api/auth/logout", post(api::auth::logout))
        .route("/api/auth/session", get(api::auth::get_session))
        .route("/api/auth/events", get(api::auth::get_my_auth_events))
        .route(
            "/api/auth/events/recent",
            get(api::auth::get_recent_auth_events),
        )
        .route(
            "/api/oidc-providers",
            get(api::oidc_providers::list_oidc_providers),
        )
        .route(
            "/api/oidc-providers",
            post(api::oidc_providers::create_oidc_provider),
        )
        .route(
            "/api/oidc-providers/:id",
            get(api::oidc_providers::get_oidc_provider),
        )
        .route(
            "/api/oidc-providers/:id",
            patch(api::oidc_providers::update_oidc_provider),
        )
        .route(
            "/api/oidc-providers/:id",
            delete(api::oidc_providers::delete_oidc_provider),
        )
        // API Routes
        .route("/api/agents/:id", get(api::agents::get_agent))
        .route("/api/agents/:id", patch(api::agents::update_agent))
        .route("/api/agents/:id", delete(api::agents::delete_agent))
        .route(
            "/api/agents/:id/password",
            post(api::agents::change_agent_password),
        )
        // API Key routes (Feature 015)
        .route(
            "/api/agents/:id/api-key",
            post(api::api_keys::generate_api_key_handler),
        )
        .route(
            "/api/agents/:id/api-key",
            delete(api::api_keys::revoke_api_key_handler),
        )
        .route("/api/api-keys", get(api::api_keys::list_api_keys_handler))
        .route("/api/contacts", get(api::contacts::list_contacts))
        .route("/api/contacts", post(api::contacts::create_contact))
        .route("/api/contacts/:id", get(api::contacts::get_contact))
        .route("/api/contacts/:id", patch(api::contacts::update_contact))
        .route("/api/contacts/:id", delete(api::contacts::delete_contact))
        .route(
            "/api/conversations",
            get(api::conversations::list_conversations),
        )
        .route(
            "/api/conversations",
            post(api::conversations::create_conversation),
        )
        .route(
            "/api/conversations/:id",
            get(api::conversations::get_conversation),
        )
        .route(
            "/api/conversations/:id/status",
            patch(api::conversations::update_conversation_status),
        )
        .route(
            "/api/conversations/:id/priority",
            patch(api::conversations::update_conversation_priority),
        )
        .route(
            "/api/conversations/ref/:reference_number",
            get(api::conversations::get_conversation_by_reference),
        )
        .route("/api/roles", get(api::roles::list_roles))
        .route("/api/roles", post(api::roles::create_role))
        .route("/api/roles/:id", get(api::roles::get_role))
        .route("/api/roles/:id", patch(api::roles::update_role))
        .route("/api/roles/:id", delete(api::roles::delete_role))
        .route("/api/permissions", get(api::roles::list_permissions))
        .route("/api/users", get(api::users::list_users))
        .route("/api/users/:id", get(api::users::get_user))
        .route("/api/users/:id", delete(api::users::delete_user))
        // Team routes
        .route("/api/teams", post(api::teams::create_team))
        .route("/api/teams", get(api::teams::list_teams))
        .route("/api/teams/:id", get(api::teams::get_team))
        .route("/api/teams/:id/members", post(api::teams::add_team_member))
        .route("/api/teams/:id/members", get(api::teams::get_team_members))
        .route(
            "/api/teams/:id/members/:user_id",
            delete(api::teams::remove_team_member),
        )
        // Assignment routes
        .route(
            "/api/conversations/:id/assign",
            post(api::assignments::assign_conversation),
        )
        .route(
            "/api/conversations/:id/unassign",
            post(api::assignments::unassign_conversation),
        )
        .route(
            "/api/conversations/unassigned",
            get(api::assignments::get_unassigned_conversations),
        )
        .route(
            "/api/conversations/assigned",
            get(api::assignments::get_assigned_conversations),
        )
        .route(
            "/api/teams/:id/conversations",
            get(api::assignments::get_team_conversations),
        )
        .route(
            "/api/agents/:id/availability",
            put(api::assignments::update_agent_availability),
        )
        // Tag management routes
        .route("/api/tags", post(api::tags::create_tag))
        .route("/api/tags", get(api::tags::list_tags))
        .route("/api/tags/:id", get(api::tags::get_tag))
        .route("/api/tags/:id", patch(api::tags::update_tag))
        .route("/api/tags/:id", delete(api::tags::delete_tag))
        // Conversation tagging routes
        .route(
            "/api/conversations/:id/tags",
            get(api::conversation_tags::get_conversation_tags),
        )
        .route(
            "/api/conversations/:id/tags",
            post(api::conversation_tags::add_tags_to_conversation),
        )
        .route(
            "/api/conversations/:id/tags/:tag_id",
            delete(api::conversation_tags::remove_tag_from_conversation),
        )
        .route(
            "/api/conversations/:id/tags",
            put(api::conversation_tags::replace_conversation_tags),
        )
        // Agent availability routes
        .route(
            "/api/agents/:id/availability",
            post(api::availability::set_availability),
        )
        .route(
            "/api/agents/:id/availability",
            get(api::availability::get_availability),
        )
        .route(
            "/api/agents/:id/activity",
            get(api::availability::get_activity_log),
        )
        // SLA routes
        .route("/api/sla/policies", post(api::sla::create_sla_policy))
        .route("/api/sla/policies", get(api::sla::list_sla_policies))
        .route("/api/sla/policies/:id", get(api::sla::get_sla_policy))
        .route("/api/sla/policies/:id", put(api::sla::update_sla_policy))
        .route("/api/sla/policies/:id", delete(api::sla::delete_sla_policy))
        .route(
            "/api/sla/conversations/:conversation_id",
            get(api::sla::get_applied_sla_by_conversation),
        )
        .route("/api/sla/applied", get(api::sla::list_applied_slas))
        .route("/api/sla/apply", post(api::sla::apply_sla))
        .route(
            "/api/sla/applied/:applied_sla_id/events",
            get(api::sla::get_sla_events),
        )
        .route(
            "/api/teams/:id/sla-policy",
            put(api::sla::assign_sla_policy_to_team),
        )
        // Automation rules endpoints
        .route(
            "/api/automation/rules",
            post(api::automation::create_automation_rule),
        )
        .route(
            "/api/automation/rules",
            get(api::automation::list_automation_rules),
        )
        .route(
            "/api/automation/rules/:id",
            get(api::automation::get_automation_rule),
        )
        .route(
            "/api/automation/rules/:id",
            put(api::automation::update_automation_rule),
        )
        .route(
            "/api/automation/rules/:id",
            delete(api::automation::delete_automation_rule),
        )
        .route(
            "/api/automation/rules/:id/enable",
            patch(api::automation::enable_automation_rule),
        )
        .route(
            "/api/automation/rules/:id/disable",
            patch(api::automation::disable_automation_rule),
        )
        .route(
            "/api/automation/evaluation-logs",
            get(api::automation::list_evaluation_logs),
        )
        // Macro endpoints
        .route("/api/macros", post(api::macros::create_macro))
        .route("/api/macros", get(api::macros::list_macros))
        .route("/api/macros/:id", get(api::macros::get_macro))
        .route("/api/macros/:id", put(api::macros::update_macro))
        .route("/api/macros/:id", delete(api::macros::delete_macro))
        .route("/api/macros/:id/apply", post(api::macros::apply_macro))
        .route(
            "/api/macros/:id/access",
            post(api::macros::grant_macro_access),
        )
        .route(
            "/api/macros/:id/access",
            get(api::macros::list_macro_access),
        )
        .route(
            "/api/macros/:id/access/:entity_type/:entity_id",
            delete(api::macros::revoke_macro_access),
        )
        .route("/api/macros/:id/logs", get(api::macros::get_macro_logs))
        // Notification routes
        .route(
            "/api/notifications",
            get(api::notifications::list_notifications),
        )
        .route(
            "/api/notifications/unread-count",
            get(api::notifications::get_unread_count),
        )
        .route(
            "/api/notifications/stream",
            get(api::notifications::notification_stream),
        )
        .route(
            "/api/notifications/:id/read",
            put(api::notifications::mark_notification_as_read),
        )
        .route(
            "/api/notifications/read-all",
            put(api::notifications::mark_all_notifications_as_read),
        )
        // Inbox email configuration routes (Feature 021)
        .route(
            "/api/inboxes/:inbox_id/email-config",
            post(api::inbox_email_configs::create_inbox_email_config),
        )
        .route(
            "/api/inboxes/:inbox_id/email-config",
            get(api::inbox_email_configs::get_inbox_email_config),
        )
        .route(
            "/api/inboxes/:inbox_id/email-config",
            put(api::inbox_email_configs::update_inbox_email_config),
        )
        .route(
            "/api/inboxes/:inbox_id/email-config",
            delete(api::inbox_email_configs::delete_inbox_email_config),
        )
        .route(
            "/api/inboxes/email-config/test",
            post(api::inbox_email_configs::test_inbox_email_config),
        )
        // Webhook routes (admin only)
        .route("/api/webhooks", post(api::webhooks::create_webhook))
        .route("/api/webhooks", get(api::webhooks::list_webhooks))
        .route("/api/webhooks/:id", get(api::webhooks::get_webhook))
        .route("/api/webhooks/:id", put(api::webhooks::update_webhook))
        .route("/api/webhooks/:id", delete(api::webhooks::delete_webhook))
        .route(
            "/api/webhooks/:id/toggle",
            put(api::webhooks::toggle_webhook_status),
        )
        .route("/api/webhooks/:id/test", post(api::webhooks::test_webhook))
        .route(
            "/api/webhooks/:id/deliveries",
            get(api::webhooks::list_webhook_deliveries),
        )
        // Add activity tracking middleware (before auth middleware)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            api::middleware::track_activity_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            api::middleware::require_auth,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            api::middleware::api_key_auth_middleware,
        ));

    // Build web routes (require auth via cookie)
    let web_protected = Router::new()
        .route("/dashboard", get(web::show_dashboard))
        .route("/logout", post(web::handle_logout))
        .route("/agents", get(web::show_agents).post(web::create_agent))
        .route("/agents/new", get(web::show_create_agent_page))
        .route("/agents/:id", delete(web::delete_agent))
        .route(
            "/contacts",
            get(web::show_contacts).post(web::create_contact),
        )
        .route("/contacts/new", get(web::show_create_contact_page))
        .route(
            "/contacts/:id",
            delete(web::delete_contact)
                .get(web::show_contact_profile)
                .post(web::update_contact),
        )
        .route("/contacts/:id/edit", get(web::show_contact_edit))
        .route("/roles", get(web::show_roles))
        .route("/roles/:id", delete(web::delete_role))
        // Inbox
        .route("/inbox", get(web::show_inbox))
        .route("/inbox/c/:id", get(web::show_conversation))
        .route("/inbox/c/:id/messages", post(web::send_message))
        // Standardized routes (aliased for compatibility/template usage)
        .route("/inbox/conversations/:id", get(web::show_conversation))
        .route("/inbox/conversations/:id/assign", patch(web::assign_ticket))
        .route(
            "/inbox/conversations/:id/resolve",
            post(web::resolve_ticket),
        )
        .route("/inbox/conversations/:id/open", post(web::reopen_ticket))
        // Manual Ticket Creation
        .route("/conversations/new", get(web::show_create_ticket_page))
        .route("/conversations", post(web::create_ticket))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            web_auth_middleware,
        ));

    // Build public routes
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/health", get(health_handler))
        .route("/login", get(web::show_login_page))
        .route("/login", post(web::handle_login))
        .route("/api/auth/login", post(api::auth::login))
        .route(
            "/api/auth/oidc/providers",
            get(api::oidc_providers::list_enabled_oidc_providers),
        )
        .route(
            "/api/auth/oidc/:provider_name/login",
            get(api::auth::oidc_login),
        )
        .route("/api/auth/oidc/callback", get(api::auth::oidc_callback))
        // Password Reset routes (Feature 017) - Public endpoints
        .route(
            "/api/password-reset/request",
            post(api::password_reset::request_password_reset),
        )
        .route(
            "/api/password-reset/reset",
            post(api::password_reset::reset_password),
        )
        .merge(protected)
        .merge(web_protected)
        .merge(api::messages::routes())
        .with_state(state.clone());

    // Start notification cleanup background task
    {
        let cleanup_db = db.clone();
        tokio::spawn(async move {
            use tokio::time::{interval, Duration};
            let mut cleanup_interval = interval(Duration::from_secs(24 * 60 * 60)); // 24 hours

            tracing::info!(
                "Notification cleanup task started (24-hour interval, 30-day retention)"
            );

            loop {
                cleanup_interval.tick().await;

                match oxidesk::NotificationService::cleanup_old_notifications(&cleanup_db, Some(30))
                    .await
                {
                    Ok(count) => {
                        tracing::info!(
                            "Notification cleanup completed: {} old notifications deleted",
                            count
                        );
                    }
                    Err(e) => {
                        tracing::error!("Notification cleanup failed: {}", e);
                    }
                }
            }
        });
    }

    // Spawn email polling worker (Feature 021)

    // Create contact service factory
    let db_for_factory = db.clone();
    let contact_service_factory = move || {
        oxidesk::services::ContactService::new(
            std::sync::Arc::new(db_for_factory.clone()),
            std::sync::Arc::new(db_for_factory.clone()),
        )
    };

    let _email_polling_handle = oxidesk::spawn_email_polling_worker(
        email_repo,
        conversation_repo,
        message_repo,
        attachment_repo,
        contact_service_factory,
        attachment_storage_path,
    );
    tracing::info!("Email polling worker started");

    // Start server
    let addr = config.server_address();
    tracing::info!("Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn root_handler() -> &'static str {
    "Oxidesk User Management System"
}

async fn health_handler() -> &'static str {
    "OK"
}

use axum::{extract::Request, middleware::Next, response::Response};

/// Web authentication middleware that checks session cookie
async fn web_auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, axum::response::Redirect> {
    // Get session token from cookie
    let cookies = request
        .headers()
        .get("Cookie")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let token = cookies.split(';').find_map(|cookie| {
        let cookie = cookie.trim();
        if cookie.starts_with("session_token=") {
            Some(cookie.trim_start_matches("session_token="))
        } else {
            None
        }
    });

    let token = match token {
        Some(t) => t.to_string(), // Clone to owned string
        None => return Err(axum::response::Redirect::to("/login")),
    };

    // Validate session
    let session = match state.session_service.get_session_by_token(&token).await {
        Ok(Some(s)) => s,
        _ => return Err(axum::response::Redirect::to("/login")),
    };

    if session.is_expired() {
        let _ = state.session_service.delete_session(&token).await;
        return Err(axum::response::Redirect::to("/login"));
    }

    // Get user
    let user = match state.db.get_user_by_id(&session.user_id).await {
        Ok(Some(u)) => u,
        _ => return Err(axum::response::Redirect::to("/login")),
    };

    // Only agents can authenticate
    if !matches!(user.user_type, UserType::Agent) {
        return Err(axum::response::Redirect::to("/login"));
    }

    // Get agent
    let agent = match state.db.get_agent_by_user_id(&user.id).await {
        Ok(Some(a)) => a,
        _ => return Err(axum::response::Redirect::to("/login")),
    };

    // Get roles
    let roles = match state.db.get_user_roles(&user.id).await {
        Ok(r) => r,
        _ => return Err(axum::response::Redirect::to("/login")),
    };

    // Compute permissions from all roles
    let mut permissions = std::collections::HashSet::new();
    for role in &roles {
        for permission in &role.permissions {
            permissions.insert(permission.clone());
        }
    }
    let permissions: Vec<String> = permissions.into_iter().collect();

    // Store authenticated user in request extensions
    request
        .extensions_mut()
        .insert(api::middleware::AuthenticatedUser {
            user,
            agent,
            roles,
            permissions,
            session,
            token,
        });

    Ok(next.run(request).await)
}

async fn initialize_admin(db: &Database, config: &Config) -> Result<(), ApiError> {
    tracing::info!("Checking for admin user initialization");

    // Check if admin already exists
    if let Some(_) = db
        .get_user_by_email_and_type(&config.admin_email, &UserType::Agent)
        .await?
    {
        tracing::info!("Admin user already exists: {}", config.admin_email);
        return Ok(());
    }

    tracing::info!("Creating admin user: {}", config.admin_email);

    // Validate admin password complexity
    validate_password_complexity(&config.admin_password)?;

    // Validate and normalize email
    let email = validate_and_normalize_email(&config.admin_email)?;

    // Hash password
    let password_hash = hash_password(&config.admin_password)?;

    // Create user
    let user = User::new(email, UserType::Agent);
    db.create_user(&user).await?;

    // Create agent
    let agent = Agent::new(user.id.clone(), "Admin".to_string(), None, password_hash);
    db.create_agent(&agent).await?;

    // Get Admin role
    let admin_role = db
        .get_role_by_name("Admin")
        .await?
        .ok_or_else(|| ApiError::Internal("Admin role not found in seed data".to_string()))?;

    // Assign Admin role
    let user_role = UserRole::new(user.id.clone(), admin_role.id);
    db.assign_role_to_user(&user_role).await?;

    tracing::info!("Admin user created successfully: {}", config.admin_email);

    Ok(())
}
