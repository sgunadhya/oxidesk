use crate::application::services::auth::{hash_password, validate_password_complexity};
use crate::application::services::automation_service::AutomationConfig;
use crate::application::services::*;
use crate::config::Config;
use crate::domain::entities::*;
use crate::domain::ports::agent_repository::AgentRepository;
use crate::domain::ports::api_key_repository::ApiKeyRepository;
use crate::domain::ports::assignment_repository::AssignmentRepository;
use crate::domain::ports::automation_repository::AutomationRepository;
use crate::domain::ports::availability_repository::AvailabilityRepository;
use crate::domain::ports::conversation_repository::ConversationRepository;
use crate::domain::ports::conversation_tag_repository::ConversationTagRepository;
use crate::domain::ports::inbox_repository::InboxRepository;
use crate::domain::ports::message_repository::MessageRepository;
use crate::domain::ports::notification_repository::NotificationRepository;
use crate::domain::ports::oidc_repository::OidcRepository;
use crate::domain::ports::role_repository::RoleRepository;
use crate::domain::ports::tag_repository::TagRepository;
use crate::domain::ports::team_repository::TeamRepository;
use crate::domain::ports::user_repository::UserRepository;
use crate::domain::ports::webhook_repository::WebhookRepository;
use crate::infrastructure::http::middleware::{ApiError, AppState};
use crate::infrastructure::persistence::Database;
use crate::infrastructure::providers::connection_manager::{
    ConnectionManager, InMemoryConnectionManager,
};
use crate::infrastructure::workers::job_queue::TaskQueue;
use crate::shared::utils::email_validator::validate_and_normalize_email;
use crate::LocalEventBus;
use std::sync::Arc;

pub async fn build_app_state(
    db: Database,
    config: &Config,
) -> Result<AppState, Box<dyn std::error::Error>> {
    let attachment_storage_path =
        std::env::var("ATTACHMENT_STORAGE_PATH").unwrap_or_else(|_| "./attachments".to_string());
    std::fs::create_dir_all(&attachment_storage_path)?;

    // Initialize TaskSpawner
    let task_spawner = Arc::new(crate::infrastructure::runtime::tokio::TokioTaskSpawner::new())
        as Arc<dyn crate::domain::ports::task_spawner::TaskSpawner>;

    // Initialize event bus for automation rules
    let event_bus = std::sync::Arc::new(LocalEventBus::new(100));
    tracing::info!("Event bus initialized with capacity 100");

    // Initialize delivery service with mock provider
    let delivery_provider = std::sync::Arc::new(
        crate::application::services::delivery_service::MockDeliveryProvider::new(),
    );
    let delivery_service = crate::application::services::DeliveryService::new(
        Arc::new(db.clone()) as Arc<dyn MessageRepository>,
        delivery_provider,
    );
    tracing::info!("Delivery service initialized with mock provider");

    // Initialize notification service
    let notification_repo: Arc<dyn NotificationRepository> = Arc::new(db.clone());
    let notification_service = crate::NotificationService::new(Some(notification_repo));
    tracing::info!("Notification service initialized");

    // Initialize availability service
    let availability_service = crate::AvailabilityService::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn AgentRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn AvailabilityRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn ConversationRepository>,
        event_bus.clone(),
    );
    tracing::info!("Availability service initialized");

    // Initialize SLA service
    let sla_service = crate::SlaService::new(
        std::sync::Arc::new(db.clone())
            as std::sync::Arc<dyn crate::domain::ports::sla_repository::SlaRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn ConversationRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn TeamRepository>,
        event_bus.clone(),
    );
    tracing::info!("SLA service initialized");

    // Initialize Tag Repository (needed by ActionExecutor)
    let tag_repo = TagRepository::new(db.clone());

    // Initialize automation service
    let action_executor = crate::domain::services::action_executor::ActionExecutor::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn ConversationRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn UserRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn AgentRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn TeamRepository>,
        tag_repo.clone(),
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn ConversationTagRepository>,
    );
    let automation_service = std::sync::Arc::new(crate::AutomationService::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn AutomationRepository>,
        action_executor,
        AutomationConfig::default(),
    ));
    // Initialize webhook service
    let webhook_repo = WebhookRepository::new(db.clone());
    let webhook_service = crate::WebhookService::new(webhook_repo);

    // Initialize Conversation Tag Service
    let conversation_tag_service = ConversationTagService::new(
        Arc::new(db.clone()) as Arc<dyn ConversationTagRepository>,
        tag_repo.clone(),
        Arc::new(db.clone()) as Arc<dyn ConversationRepository>,
        event_bus.clone(),
    );
    tracing::info!("Conversation tag service initialized");

    // Initialize Conversation Priority Service
    let conversation_priority_service = crate::ConversationPriorityService::new(
        Arc::new(db.clone()) as Arc<dyn ConversationRepository>,
        Some(event_bus.clone()),
    );
    tracing::info!("Conversation priority service initialized");

    let inbox_service = InboxService::new(Arc::new(db.clone()) as Arc<dyn InboxRepository>);
    let tag_service = TagService::new(tag_repo.clone());
    let role_service = RoleService::new(Arc::new(db.clone()) as Arc<dyn RoleRepository>);
    tracing::info!("Automation service initialized");
    let connection_manager: Arc<dyn ConnectionManager> = Arc::new(InMemoryConnectionManager::new());
    tracing::info!("Connection manager initialized");

    // Initialize rate limiter
    let rate_limiter = crate::shared::rate_limiter::AuthRateLimiter::new();
    tracing::info!("Rate limiter initialized (5 attempts per 15 minutes)");

    // Initialize TaskQueue
    let task_queue = std::sync::Arc::new(crate::infrastructure::workers::SqliteTaskQueue::new(
        db.clone(),
    ));

    // Enqueue initial maintenance jobs
    let q_init = task_queue.clone();
    task_spawner.spawn(Box::pin(async move {
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
    }));

    // Initialize Session Service
    let session_service =
        crate::application::services::SessionService::new(std::sync::Arc::new(db.clone()));
    tracing::info!("Session service initialized");

    // Initialize Agent Service
    let agent_service = crate::application::services::AgentService::new(
        std::sync::Arc::new(db.clone()) as Arc<dyn AgentRepository>,
        std::sync::Arc::new(db.clone()) as Arc<dyn ApiKeyRepository>,
        std::sync::Arc::new(db.clone()) as Arc<dyn UserRepository>,
        std::sync::Arc::new(db.clone()) as Arc<dyn RoleRepository>,
        session_service.clone(),
    );
    tracing::info!("Agent service initialized");

    // Initialize User Service
    let user_service =
        crate::application::services::UserService::new(std::sync::Arc::new(db.clone()));
    tracing::info!("User service initialized");

    // Initialize Contact Service
    let contact_service = crate::application::services::ContactService::new(
        std::sync::Arc::new(db.clone()),
        std::sync::Arc::new(db.clone()),
    );
    tracing::info!("Contact service initialized");

    // Initialize Repositories
    let email_repo: std::sync::Arc<dyn crate::domain::ports::email_repository::EmailRepository> =
        std::sync::Arc::new(db.clone());
    let attachment_repo: std::sync::Arc<
        dyn crate::domain::ports::attachment_repository::AttachmentRepository,
    > = std::sync::Arc::new(db.clone());
    let conversation_repo: std::sync::Arc<dyn ConversationRepository> =
        std::sync::Arc::new(db.clone());
    let message_repo: std::sync::Arc<dyn MessageRepository> = std::sync::Arc::new(db.clone());
    let user_repo: std::sync::Arc<dyn UserRepository> = std::sync::Arc::new(db.clone());
    let contact_repo: std::sync::Arc<
        dyn crate::domain::ports::contact_repository::ContactRepository,
    > = std::sync::Arc::new(db.clone());
    let team_repo: std::sync::Arc<dyn TeamRepository> = std::sync::Arc::new(db.clone());
    let team_service = crate::application::services::TeamService::new(team_repo.clone());

    // Initialize Assignment Service
    let assignment_repo: Arc<dyn AssignmentRepository> = Arc::new(db.clone());
    let availability_repo_for_assignment: Arc<dyn AvailabilityRepository> = Arc::new(db.clone());
    let assignment_service = {
        let mut service = crate::application::services::AssignmentService::new(
            assignment_repo,
            conversation_repo.clone(),
            Arc::new(db.clone()) as Arc<dyn AgentRepository>,
            user_repo.clone(),
            Arc::new(db.clone()) as Arc<dyn RoleRepository>,
            team_repo.clone(),
            availability_repo_for_assignment,
            event_bus.clone(),
            notification_service.clone(),
            connection_manager.clone(),
        );
        service.set_sla_service(Arc::new(sla_service.clone()));
        service
    };
    tracing::info!("Assignment service initialized");

    // Initialize OIDC service
    let oidc_repository = OidcRepository::new(db.clone());
    let oidc_service = crate::application::services::OidcService::new(
        oidc_repository,
        user_repo.clone(),
        Arc::new(db.clone()) as Arc<dyn AgentRepository>,
        Arc::new(db.clone()) as Arc<dyn RoleRepository>,
    );
    tracing::info!("OIDC service initialized");

    // Initialize Services (wrapping repositories)
    let email_service = crate::application::services::EmailService::new(email_repo.clone());
    let attachment_service = crate::application::services::AttachmentService::new(
        attachment_repo.clone(),
        std::path::PathBuf::from(&attachment_storage_path),
    );

    let conversation_service = crate::application::services::ConversationService::new(
        conversation_repo.clone(),
        user_repo.clone(),
        contact_repo.clone(),
        team_repo.clone(),
    );

    let message_service = crate::application::services::MessageService::with_all_services(
        message_repo.clone(),
        conversation_repo.clone(),
        delivery_service.clone(),
        event_bus.clone(),
        connection_manager.clone(),
    );

    // Initialize MacroService
    let macro_repo = crate::domain::ports::macro_repository::MacroRepository::new(db.clone());
    let macro_service = crate::application::services::MacroService::new(macro_repo);

    // Initialize AuthService
    let auth_service = crate::application::services::AuthService::new(
        Arc::new(db.clone()) as Arc<dyn UserRepository>,
        Arc::new(db.clone()) as Arc<dyn AgentRepository>,
        Arc::new(db.clone()) as Arc<dyn RoleRepository>,
        session_service.clone(),
    );

    // Initialize PasswordResetService
    let password_reset_repo =
        crate::domain::ports::password_reset_repository::PasswordResetRepository::new(db.clone());
    let password_reset_service = crate::application::services::PasswordResetService::new(
        password_reset_repo,
        Arc::new(db.clone()) as Arc<dyn UserRepository>,
    );
    tracing::info!("Password reset service initialized");

    // Initialize AuthLoggerService
    let auth_logger_service =
        crate::application::services::AuthLoggerService::new(Arc::new(db.clone()));
    tracing::info!("Auth logger service initialized");

    // Start automation listener background task
    let automation_event_bus = event_bus.clone();
    let automation_sla_svc = sla_service.clone();
    let automation_svc = automation_service.clone();
    let automation_conv_svc = conversation_service.clone();
    let automation_team_svc = team_service.clone();

    task_spawner.spawn(Box::pin(async move {
        crate::application::listeners::automation::run_automation_listener(
            automation_event_bus,
            automation_sla_svc,
            automation_svc,
            automation_conv_svc,
            automation_team_svc,
        )
        .await;
    }));

    // Start webhook worker background task
    let webhook_repo_for_worker = WebhookRepository::new(db.clone());
    let webhook_event_bus = event_bus.clone();
    // Pass task_queue to WebhookWorker
    let webhook_worker = crate::infrastructure::workers::WebhookWorker::new(
        webhook_repo_for_worker,
        webhook_event_bus,
        task_queue.clone(),
    );
    task_spawner.spawn(Box::pin(async move {
        webhook_worker.run().await;
    }));
    tracing::info!("Webhook worker started");

    // Start notification cleanup background task
    {
        let cleanup_db = db.clone();
        task_spawner.spawn(Box::pin(async move {
            use tokio::time::{interval, Duration};
            let mut cleanup_interval = interval(Duration::from_secs(24 * 60 * 60)); // 24 hours

            tracing::info!(
                "Notification cleanup task started (24-hour interval, 30-day retention)"
            );

            loop {
                cleanup_interval.tick().await;

                match crate::NotificationService::cleanup_old_notifications(&cleanup_db, Some(30))
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
        }));
    }

    // Initialize TimeService
    let time_service =
        std::sync::Arc::new(crate::infrastructure::runtime::tokio::TokioTimeService::new());

    // Spawn email polling worker (Feature 021)
    let db_clone_for_email = db.clone();
    let email_worker = crate::infrastructure::providers::email_receiver::EmailPollingWorker::new(
        email_repo.clone(),
        conversation_repo.clone(),
        message_repo.clone(),
        attachment_repo.clone(),
        move || {
            crate::application::services::ContactService::new(
                std::sync::Arc::new(db_clone_for_email.clone()),
                std::sync::Arc::new(db_clone_for_email.clone()),
            )
        },
        attachment_storage_path.clone(),
        time_service.clone(),
    );
    task_spawner.spawn(Box::pin(async move {
        email_worker.run().await;
    }));
    tracing::info!("Email polling worker started");

    // Spawn JobProcessor for background tasks
    let oidc_repo = OidcRepository::new(db.clone());
    let webhook_repo = WebhookRepository::new(db.clone());
    let job_processor = crate::infrastructure::workers::job_worker::JobProcessor::new(
        task_queue.clone(),
        oidc_repo.clone(),
        webhook_repo.clone(),
        rate_limiter.clone(),
        availability_service.clone(),
        sla_service.clone(),
        session_service.clone(),
        time_service.clone(),
    );
    task_spawner.spawn(Box::pin(async move {
        job_processor.run().await;
    }));

    // Create application state
    Ok(AppState {
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
        assignment_service: assignment_service.clone(),
        auth_logger_service,
    })
}

pub async fn initialize_admin(db: &Database, config: &Config) -> Result<(), ApiError> {
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
