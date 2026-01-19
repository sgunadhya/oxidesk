use crate::infrastructure::http as api;
use crate::infrastructure::http::middleware::{
    api_key_auth_middleware, require_auth, track_activity_middleware, web_auth_middleware, AppState,
};
use crate::infrastructure::web;
use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};

pub fn build_router(state: AppState) -> Router {
    // Build protected routes (require authentication)
    let protected = Router::new()
        .route("/api/auth/logout", post(api::controllers::auth::logout))
        .route(
            "/api/auth/session",
            get(api::controllers::auth::get_session),
        )
        .route(
            "/api/auth/events",
            get(api::controllers::auth::get_my_auth_events),
        )
        .route(
            "/api/auth/events/recent",
            get(api::controllers::auth::get_recent_auth_events),
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
            track_activity_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            require_auth,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            api_key_auth_middleware,
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
    Router::new()
        .route("/", get(root_handler))
        .route("/health", get(health_handler))
        .route("/login", get(web::show_login_page))
        .route("/login", post(web::handle_login))
        .route("/api/auth/login", post(api::controllers::auth::login))
        .route(
            "/api/auth/oidc/providers",
            get(api::oidc_providers::list_enabled_oidc_providers),
        )
        .route(
            "/api/auth/oidc/:provider_name/login",
            get(api::controllers::auth::oidc_login),
        )
        .route(
            "/api/auth/oidc/callback",
            get(api::controllers::auth::oidc_callback),
        )
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
        .with_state(state)
}

async fn root_handler() -> &'static str {
    "Oxidesk User Management System"
}

async fn health_handler() -> &'static str {
    "OK"
}
