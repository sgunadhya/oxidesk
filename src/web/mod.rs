use crate::database::agents::AgentRepository;
use crate::{
    api::middleware::{AppState, AuthenticatedUser},
    models::CreateAgentRequest,
    services,
};
use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;

// Template structs
#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {}

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate {
    user_name: String,
    user_email: String,
    user_roles: Vec<String>,
    stats: DashboardStats,
    request_path: String,
}

struct DashboardStats {
    total_agents: i64,
    total_contacts: i64,
    total_roles: usize,
}

#[derive(Template)]
#[template(path = "agents.html")]
struct AgentsTemplate {
    agents: Vec<AgentData>,
    roles: Vec<RoleData>,
    request_path: String,
}

#[derive(Template)]
#[template(path = "agents_new.html")]
struct AgentsNewTemplate {
    roles: Vec<RoleData>,
    request_path: String,
}

struct AgentData {
    id: String,
    email: String,
    first_name: String,
    roles: Vec<String>,
    created_at: String,
}

#[derive(Template)]
#[template(path = "contacts.html")]
struct ContactsTemplate {
    contacts: Vec<ContactData>,
    request_path: String,
}

#[derive(Template)]
#[template(path = "contacts_new.html")]
struct ContactsNewTemplate {
    request_path: String,
}

struct ContactData {
    id: String,
    email: String,
    full_name: String,
    channel_count: usize,
    created_at: String,
}

#[derive(Template)]
#[template(path = "contacts_edit.html")]
struct ContactEditTemplate {
    contact: ContactData,
    request_path: String,
}

#[derive(Template)]
#[template(path = "conversations_new.html")]
struct ConversationsNewTemplate {
    contacts: Vec<ContactData>,
    request_path: String,
}

#[derive(Template)]
#[template(path = "roles.html")]
struct RolesTemplate {
    roles: Vec<RoleData>,
    request_path: String,
}

struct RoleData {
    id: String,
    name: String,
    description: String,
    permissions: Vec<String>,
    user_count: i64,
    is_system: bool,
}

#[derive(Template)]
#[template(path = "partials/agent_row.html")]
struct AgentRowTemplate {
    agent: AgentData,
}

#[derive(Template)]
#[template(path = "inbox.html")]
struct InboxTemplate {
    conversations: Vec<ConversationData>,
    selected_id: Option<String>,
    conversation: ConversationDetailData,
    messages: Vec<MessageData>,
    agents: Vec<AgentData>,
    request_path: String,
}

#[derive(Template)]
#[template(path = "partials/conversation_list.html")]
struct ConversationListPartial {
    conversations: Vec<ConversationData>,
    selected_id: Option<String>,
}

#[derive(Template)]
#[template(path = "partials/conversation_detail.html")]

struct ConversationDetailPartial {
    conversation: ConversationDetailData,
    messages: Vec<MessageData>,
    agents: Vec<AgentData>,
}

struct ConversationData {
    id: String,
    contact_name: String,
    subject: String,
    status: String,
    updated_at: String,
}

struct ConversationDetailData {
    id: String,
    subject: String,
    contact_name: String,
    status: String,
    assigned_user_id: Option<String>,
}

struct MessageData {
    id: String,
    sender_name: String,
    content: String,
    is_agent: bool,
    created_at: String,
}

#[derive(Template)]
#[template(path = "contact_profile.html")]
struct ContactProfileTemplate {
    contact: ContactData,
    channels: Vec<ChannelData>,
    conversations: Vec<ConversationData>,
    request_path: String,
}

struct ChannelData {
    email: String,
    inbox_id: String,
}

#[derive(Template)]
#[template(path = "partials/error.html")]
struct ErrorPartial {
    message: String,
}

// Form data
#[derive(Deserialize)]
pub struct LoginForm {
    email: String,
    password: String,
}

#[derive(Deserialize)]
pub struct CreateAgentForm {
    first_name: String,
    last_name: Option<String>,
    email: String,
    role_id: String,
}

#[derive(Deserialize)]
pub struct CreateContactForm {
    email: String,
    full_name: Option<String>,
}

#[derive(Deserialize)]
pub struct ContactUpdateForm {
    full_name: String,
    email: String,
}

#[derive(Deserialize)]
pub struct CreateTicketForm {
    subject: String,
    contact_id: String,
    // inbox_id: Option<String>, // Future: Allow selecting inbox
}

#[derive(Deserialize)]
pub struct SendMessageForm {
    content: String,
}

// Handlers
pub async fn show_login_page() -> impl IntoResponse {
    let template = LoginTemplate {};
    HtmlTemplate(template)
}

pub async fn handle_login(State(state): State<AppState>, Form(form): Form<LoginForm>) -> Response {
    // Delegate to auth service
    let auth_result = match services::auth::authenticate(
        &state.db,
        &form.email,
        &form.password,
        state.session_duration_hours,
    )
    .await
    {
        Ok(result) => result,
        Err(_) => {
            tracing::warn!("Login failed for {}", form.email);
            return Html("<div class=\"alert alert-error\">Invalid email or password</div>")
                .into_response();
        }
    };

    tracing::info!("Login successful for user {}", auth_result.user.email);

    // Set session cookie and redirect to dashboard
    (
        StatusCode::OK,
        [
            (
                "Set-Cookie",
                format!(
                    "session_token={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
                    auth_result.session.token,
                    state.session_duration_hours * 3600
                ),
            ),
            ("HX-Redirect", "/dashboard".to_string()),
        ],
    )
        .into_response()
}

pub async fn show_dashboard(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    // Get stats
    let total_agents = state.db.count_agents().await.unwrap_or(0);
    let total_contacts = state.db.count_contacts().await.unwrap_or(0);
    let roles = state.db.list_roles().await.unwrap_or_default();

    let user_role_names: Vec<String> = auth_user.roles.iter().map(|r| r.name.clone()).collect();

    let template = DashboardTemplate {
        user_name: auth_user.agent.first_name.clone(),
        user_email: auth_user.user.email.clone(),
        user_roles: user_role_names,
        stats: DashboardStats {
            total_agents,
            total_contacts,
            total_roles: roles.len(),
        },
        request_path: "/dashboard".to_string(),
    };

    HtmlTemplate(template)
}

pub async fn handle_logout(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    tracing::info!(
        "Logout for user {} ({})",
        auth_user.user.email,
        auth_user.user.id
    );

    // Delete session
    let _ = state.db.delete_session(&auth_user.token).await;

    // Clear cookie and redirect
    (
        StatusCode::OK,
        [(
            "Set-Cookie",
            "session_token=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0",
        )],
        Redirect::to("/login"),
    )
}

pub async fn show_agents(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    // Get all agents (with large limit for now, proper pagination TODO)
    let agents = match state.db.list_agents(1000, 0).await {
        Ok(agents) => agents,
        Err(_) => {
            return Html("<div class=\"alert alert-error\">Failed to load agents</div>")
                .into_response();
        }
    };

    // Build agent data with roles
    let mut agent_data = Vec::new();
    for (user, agent) in agents {
        let roles = match state.db.get_user_roles(&user.id).await {
            Ok(r) => r,
            _ => vec![],
        };

        let role_names: Vec<String> = roles.iter().map(|r| r.name.clone()).collect();

        agent_data.push(AgentData {
            id: user.id, // Use user_id for deletion
            email: user.email,
            first_name: agent.first_name,
            roles: role_names,
            created_at: user.created_at,
        });
    }

    // Get all roles for the dropdown
    let all_roles = match state.db.list_roles().await {
        Ok(roles) => roles,
        Err(_) => vec![],
    };

    let role_options = all_roles
        .into_iter()
        .map(|r| RoleData {
            id: r.id,
            name: r.name,
            description: r.description.unwrap_or_default(),
            permissions: vec![], // Not needed for dropdown
            user_count: 0,       // Not needed
            is_system: false,    // Not needed
        })
        .collect();

    let template = AgentsTemplate {
        agents: agent_data,
        roles: role_options,
        request_path: "/agents".to_string(),
    };

    HtmlTemplate(template).into_response()
}

pub async fn delete_agent(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Response {
    match services::agent_service::delete(&state.db, &auth_user, &id).await {
        Ok(()) => Html("").into_response(),
        Err(e) => HtmlTemplate(ErrorPartial {
            message: e.to_string(),
        })
        .into_response(),
    }
}

pub async fn create_agent(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Form(form): Form<CreateAgentForm>,
) -> Response {
    let request = CreateAgentRequest {
        email: form.email,
        first_name: form.first_name,
        last_name: form.last_name,
        role_id: Some(form.role_id),
    };

    match services::agent_service::create_agent(&state.db, &auth_user, request).await {
        Ok(_) => {
            // Redirect to agents list with success toast
            (
                StatusCode::SEE_OTHER, // Use 303 for redirect after POST
                [
                    ("Location", "/agents"),
                    (
                        "HX-Trigger",
                        r#"{"toast": {"value": "Agent created successfully", "type": "success"}}"#,
                    ),
                ],
            )
                .into_response()
        }
        Err(e) => {
            // For now, simple error response. In a real form, we'd render the form again with errors.
            (
                StatusCode::BAD_REQUEST,
                [(
                    "HX-Trigger",
                    format!(
                        r#"{{"toast": {{"value": "Error: {}", "type": "error"}}}}"#,
                        e
                    )
                    .as_str(),
                )],
                HtmlTemplate(ErrorPartial {
                    message: e.to_string(),
                }),
            )
                .into_response()
        }
    }
}

pub async fn show_contacts(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    // Get all contacts (with large limit for now, proper pagination TODO)
    let contacts = match state.db.list_contacts(1000, 0).await {
        Ok(contacts) => contacts,
        Err(_) => {
            return Html("<div class=\"alert alert-error\">Failed to load contacts</div>")
                .into_response();
        }
    };

    // Build contact data with channels
    let mut contact_data = Vec::new();
    for (user, contact) in contacts {
        // Get channels for this contact
        let channels = match state.db.find_contact_channels(&contact.id).await {
            Ok(c) => c,
            _ => vec![],
        };

        // Build full name
        let full_name = contact.first_name.unwrap_or_else(|| String::new());

        contact_data.push(ContactData {
            id: user.id, // Use user_id for deletion
            email: user.email,
            full_name,
            channel_count: channels.len(),
            created_at: user.created_at,
        });
    }

    let template = ContactsTemplate {
        contacts: contact_data,
        request_path: "/contacts".to_string(),
    };

    HtmlTemplate(template).into_response()
}

pub async fn delete_contact(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Response {
    match services::contact_service::delete(&state.db, &auth_user, &id).await {
        Ok(()) => Html("").into_response(),
        Err(e) => HtmlTemplate(ErrorPartial {
            message: e.to_string(),
        })
        .into_response(),
    }
}

pub async fn show_roles(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    // Get all roles
    let roles = match state.db.list_roles().await {
        Ok(roles) => roles,
        Err(_) => {
            return Html("<div class=\"alert alert-error\">Failed to load roles</div>")
                .into_response();
        }
    };

    // Build role data with permissions and user counts
    let mut role_data = Vec::new();
    for role in roles {
        // Get permissions for this role
        let permissions = match state.db.get_role_permissions(&role.id).await {
            Ok(perms) => perms,
            _ => vec![],
        };

        let permission_names: Vec<String> = permissions.iter().map(|p| p.name.clone()).collect();

        // Get user count
        let user_count = match state.db.count_users_with_role(&role.id).await {
            Ok(count) => count,
            _ => 0,
        };

        // Check if system role
        let is_system = role.name == "Admin" || role.name == "Agent";

        role_data.push(RoleData {
            id: role.id,
            name: role.name,
            description: role.description.unwrap_or_default(),
            permissions: permission_names,
            user_count,
            is_system,
        });
    }

    let template = RolesTemplate {
        roles: role_data,
        request_path: "/roles".to_string(),
    };

    HtmlTemplate(template).into_response()
}

pub async fn delete_role(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Response {
    match services::role_service::delete(&state.db, &auth_user, &id).await {
        Ok(()) => Html("").into_response(),
        Err(e) => HtmlTemplate(ErrorPartial {
            message: e.to_string(),
        })
        .into_response(),
    }
}

// Helper to render Askama templates
struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template: {}", err),
            )
                .into_response(),
        }
    }
}

pub async fn show_create_agent_page(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    // Get all roles for the dropdown
    let all_roles = match state.db.list_roles().await {
        Ok(roles) => roles,
        Err(_) => vec![],
    };

    let role_options = all_roles
        .into_iter()
        .map(|r| RoleData {
            id: r.id,
            name: r.name,
            description: r.description.unwrap_or_default(),
            permissions: vec![],
            user_count: 0,
            is_system: false,
        })
        .collect();

    let template = AgentsNewTemplate {
        roles: role_options,
        request_path: "/agents".to_string(), // Keep 'Agents' active in sidebar
    };

    HtmlTemplate(template).into_response()
}

pub async fn show_create_contact_page(
    State(_state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    let template = ContactsNewTemplate {
        request_path: "/contacts".to_string(), // Keep 'Contacts' active in sidebar
    };

    HtmlTemplate(template).into_response()
}

pub async fn create_contact(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Form(form): Form<CreateContactForm>,
) -> Response {
    let request = crate::models::CreateContactRequest {
        email: form.email,
        first_name: form.full_name,
        inbox_id: String::new(), // No inbox selected for basic contact creation
    };

    match services::contact_service::create_contact(&state.db, &auth_user, request).await {
        Ok(_) => (
            StatusCode::SEE_OTHER,
            [
                ("Location", "/contacts"),
                (
                    "HX-Trigger",
                    r#"{"toast": {"value": "Contact created successfully", "type": "success"}}"#,
                ),
            ],
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            [(
                "HX-Trigger",
                format!(
                    r#"{{"toast": {{"value": "Error: {}", "type": "error"}}}}"#,
                    e
                )
                .as_str(),
            )],
            HtmlTemplate(ErrorPartial {
                message: e.to_string(),
            }),
        )
            .into_response(),
    }
}

pub async fn show_inbox(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    let conversations = match crate::services::conversation_service::list_conversations(
        &state.db, 1, 50, None, // status
        None, // inbox_id
        None, // contact_id
    )
    .await
    {
        Ok(list) => list.conversations,
        Err(_) => vec![],
    };

    let mut conversation_data = Vec::new();
    for conv in conversations {
        let contact_name = match state.db.get_user_by_id(&conv.contact_id).await {
            Ok(Some(u)) => match state.db.find_contact_by_user_id(&u.id).await {
                Ok(Some(c)) => c.first_name.unwrap_or_else(|| u.email.clone()),
                _ => u.email,
            },
            _ => "Unknown".to_string(),
        };

        conversation_data.push(ConversationData {
            id: conv.id,
            contact_name,
            subject: conv.subject.unwrap_or_default(),
            status: conv.status.to_string(),
            updated_at: conv.updated_at,
        });
    }

    let dummy_conversation = ConversationDetailData {
        id: String::new(),
        subject: String::new(),
        contact_name: String::new(),
        status: String::new(),
        assigned_user_id: None,
    };

    let template = InboxTemplate {
        conversations: conversation_data,
        selected_id: None,
        conversation: dummy_conversation,
        messages: vec![],
        agents: vec![],
        request_path: "/inbox".to_string(),
    };

    HtmlTemplate(template).into_response()
}

pub async fn show_conversation(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let is_htmx = headers.get("HX-Request").is_some();

    // Fetch conversation
    let conversation =
        match crate::services::conversation_service::get_conversation(&state.db, &id).await {
            Ok(c) => c,
            Err(_) => return Html("Conversation not found").into_response(),
        };

    // Fetch contact name for detail view
    let contact_name = match state.db.get_user_by_id(&conversation.contact_id).await {
        Ok(Some(u)) => match state.db.find_contact_by_user_id(&u.id).await {
            Ok(Some(c)) => c.first_name.unwrap_or_else(|| u.email.clone()),
            _ => u.email,
        },
        _ => "Unknown".to_string(),
    };

    // Prepare Detail Data
    let detail_data = ConversationDetailData {
        id: conversation.id.clone(),
        subject: conversation.subject.clone().unwrap_or_default(),
        contact_name: contact_name.clone(),
        status: conversation.status.to_string(),
        assigned_user_id: conversation.assigned_user_id.clone(),
    };

    // Fetch Agents for Dropdown
    let agents_list = match state.db.list_agents(1000, 0).await {
        Ok(list) => list,
        Err(_) => vec![],
    };

    let agent_data: Vec<AgentData> = agents_list
        .into_iter()
        .map(|(u, a)| AgentData {
            id: u.id,
            email: u.email,
            first_name: a.first_name,
            roles: vec![], // Not needed here
            created_at: u.created_at,
        })
        .collect();

    // Message Logic
    let message_service = crate::services::MessageService::new(state.db.clone());
    let (messages, _total) = match message_service.list_messages(&id, 1, 50).await {
        Ok(res) => res,
        Err(_) => (vec![], 0),
    };

    let mut message_data = Vec::new();
    for msg in messages {
        let (sender_name, is_agent) = if msg.author_id == conversation.contact_id {
            ("Contact".to_string(), false)
        } else {
            ("Agent".to_string(), true)
        };

        message_data.push(MessageData {
            id: msg.id,
            sender_name,
            content: msg.content,
            is_agent,
            created_at: msg.created_at,
        });
    }

    if is_htmx {
        let template = ConversationDetailPartial {
            conversation: detail_data,
            messages: message_data,
            agents: agent_data,
        };
        HtmlTemplate(template).into_response()
    } else {
        // Full page render
        let all_convs = match crate::services::conversation_service::list_conversations(
            &state.db, 1, 50, None, None, None,
        )
        .await
        {
            Ok(list) => list.conversations,
            Err(_) => vec![],
        };

        let mut conversation_data = Vec::new();
        for conv in all_convs {
            let c_name = match state.db.get_user_by_id(&conv.contact_id).await {
                Ok(Some(u)) => match state.db.find_contact_by_user_id(&u.id).await {
                    Ok(Some(c)) => c.first_name.unwrap_or_else(|| u.email.clone()),
                    _ => u.email,
                },
                _ => "Unknown".to_string(),
            };

            conversation_data.push(ConversationData {
                id: conv.id,
                contact_name: c_name,
                subject: conv.subject.unwrap_or_default(),
                status: conv.status.to_string(),
                updated_at: conv.updated_at,
            });
        }

        let template = InboxTemplate {
            conversations: conversation_data,
            selected_id: Some(id.clone()),
            conversation: detail_data,
            messages: message_data,
            agents: agent_data,
            request_path: "/inbox".to_string(),
        };
        HtmlTemplate(template).into_response()
    }
}

// Assignment Handler
#[derive(Deserialize)]
pub struct AssignTicketForm {
    agent_id: String,
}

pub async fn assign_ticket(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Form(form): Form<AssignTicketForm>,
) -> Response {
    // If agent_id is empty, unassign (TODO: Implement unassign logic if needed, for now assuming assignment)
    // Actually our dropdown has option value="" for unassign.

    if form.agent_id.is_empty() {
        // Handle unassign (omitted for brevity, requires unassign_conversation service)
        return (
            StatusCode::OK,
            [(
                "HX-Trigger",
                r#"{"toast": {"value": "Unassignment not implementing yet", "type": "warning"}}"#,
            )],
        )
            .into_response();
    }

    match crate::services::conversation_service::assign_conversation(
        &state.db,
        &id,
        &form.agent_id,
        &auth_user.user.id,
    )
    .await
    {
        Ok(_) => (
            StatusCode::OK,
            [(
                "HX-Trigger",
                r#"{"toast": {"value": "Ticket assigned successfully", "type": "success"}}"#,
            )],
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(
                "HX-Trigger",
                format!(
                    r#"{{"toast": {{"value": "Error: {}", "type": "error"}}}}"#,
                    e
                ),
            )],
        )
            .into_response(),
    }
}

// Resolution Handler
pub async fn resolve_ticket(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Response {
    // Use service to update status
    let request = crate::models::conversation::UpdateStatusRequest {
        status: crate::models::conversation::ConversationStatus::Resolved,
        snooze_duration: None,
    };

    match crate::services::conversation_service::update_conversation_status(
        &state.db, &id, request, None, None, // No event bus for now or pass context
    )
    .await
    {
        Ok(_) => {
            // We should re-render the detail view to show the new status button state
            // But returning redirect to show_conversation works for HTMX boosting if target updates
            // Or just swap the detail view.
            // Let's redirect to the conversation URL which triggers show_conversation (and returns partial if HX-Request)
            Redirect::to(&format!("/inbox/conversations/{}", id)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(
                "HX-Trigger",
                format!(
                    r#"{{"toast": {{"value": "Error: {}", "type": "error"}}}}"#,
                    e
                ),
            )],
        )
            .into_response(),
    }
}

// Re-open Handler
pub async fn reopen_ticket(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Response {
    let request = crate::models::conversation::UpdateStatusRequest {
        status: crate::models::conversation::ConversationStatus::Open,
        snooze_duration: None,
    };

    match crate::services::conversation_service::update_conversation_status(
        &state.db, &id, request, None, None,
    )
    .await
    {
        Ok(_) => Redirect::to(&format!("/inbox/conversations/{}", id)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(
                "HX-Trigger",
                format!(
                    r#"{{"toast": {{"value": "Error: {}", "type": "error"}}}}"#,
                    e
                ),
            )],
        )
            .into_response(),
    }
}

pub async fn send_message(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Form(form): Form<SendMessageForm>,
) -> impl IntoResponse {
    let request = crate::models::SendMessageRequest {
        content: form.content,
    };

    let message_service = crate::services::MessageService::with_all_services(
        state.db.clone(),
        state.delivery_service.clone(),
        state.event_bus.clone(),
        state.connection_manager.clone(),
    );

    match message_service.send_message(id.clone(), auth_user.user.id, request).await {
        Ok(msg) => {
            Html(format!(r#"
            <div class="flex justify-end">
                <div class="max-w-lg bg-indigo-600 text-white rounded-lg rounded-br-none px-4 py-2 shadow-sm">
                    <div class="text-xs text-indigo-200 mb-1">
                        Me &bull; {}
                    </div>
                    <p class="text-sm whitespace-pre-wrap">{}</p>
                </div>
            </div>
            "#, msg.created_at, msg.content)).into_response()
        },
        Err(e) => {
             Html(format!("<div class='text-red-500'>Error: {}</div>", e)).into_response()
        }
    }
}

pub async fn show_contact_profile(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>, // This is the user_id (since contacts list uses user.id)
) -> impl IntoResponse {
    // 1. Fetch User (to get email and created_at)
    let user = match state.db.get_user_by_id(&id).await {
        Ok(Some(u)) => u,
        Ok(None) => return Html("User not found").into_response(),
        Err(_) => return Html("Error fetching user").into_response(),
    };

    // 2. Fetch Contact (to get Name)
    let contact = match state.db.find_contact_by_user_id(&id).await {
        Ok(Some(c)) => c,
        Ok(None) => return Html("Contact not found").into_response(), // Should theoretically exist if listed
        Err(_) => return Html("Error fetching contact").into_response(),
    };

    // 3. Fetch Channels
    let channels = match state.db.find_contact_channels(&contact.id).await {
        Ok(c) => c,
        Err(_) => vec![],
    };

    let channel_data = channels
        .into_iter()
        .map(|c| ChannelData {
            email: c.email,
            inbox_id: c.inbox_id,
        })
        .collect();

    // 4. Fetch Conversations
    // We need a way to list conversations by contact_id. The service has this capability.
    let conversations = match crate::services::conversation_service::list_conversations(
        &state.db,
        1,
        100,              // limit
        None,             // status
        None,             // inbox_id
        Some(id.clone()), // contact_id (user_id)
    )
    .await
    {
        Ok(list) => list.conversations,
        Err(_) => vec![],
    };

    let conversation_data = conversations
        .into_iter()
        .map(|c| ConversationData {
            id: c.id,
            contact_name: String::new(), // Not needed for profile view of same contact
            subject: c.subject.unwrap_or_default(),
            status: c.status.to_string(),
            updated_at: c.updated_at,
        })
        .collect();

    let contact_data = ContactData {
        id: user.id,
        email: user.email,
        full_name: contact.first_name.unwrap_or_else(|| String::new()),
        channel_count: 0, // Not used in this view
        created_at: user.created_at,
    };

    let template = ContactProfileTemplate {
        contact: contact_data,
        channels: channel_data,
        conversations: conversation_data,
        request_path: "/contacts".to_string(),
    };

    HtmlTemplate(template).into_response()
}

pub async fn show_contact_edit(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let user = match state.db.get_user_by_id(&id).await {
        Ok(Some(u)) => u,
        Ok(None) => return Html("User not found").into_response(),
        Err(_) => return Html("Error fetching user").into_response(),
    };

    let contact = match state.db.find_contact_by_user_id(&id).await {
        Ok(Some(c)) => c,
        Ok(None) => return Html("Contact not found").into_response(),
        Err(_) => return Html("Error fetching contact").into_response(),
    };

    let contact_data = ContactData {
        id: user.id,
        email: user.email,
        full_name: contact.first_name.unwrap_or_else(|| String::new()),
        channel_count: 0,
        created_at: user.created_at,
    };

    let template = ContactEditTemplate {
        contact: contact_data,
        request_path: "/contacts".to_string(),
    };

    HtmlTemplate(template).into_response()
}

pub async fn update_contact(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Form(form): Form<ContactUpdateForm>,
) -> impl IntoResponse {
    if let Err(e) = crate::services::contact_service::update_contact_details(
        &state.db,
        &auth_user,
        &id,
        &form.full_name,
        &form.email,
    )
    .await
    {
        return Html(format!("Error updating contact: {}", e)).into_response();
    }

    // Redirect to profile
    Redirect::to(&format!("/contacts/{}", id)).into_response()
}

pub async fn show_create_ticket_page(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    let contacts = match crate::services::contact_service::list_contacts(
        &state.db, 1, 1000, // reasonable limit for dropdown
    )
    .await
    {
        Ok(list) => list.contacts,
        Err(_) => vec![],
    };

    let contact_data = contacts
        .into_iter()
        .map(|c| ContactData {
            id: c.id,
            email: c.email,
            full_name: c.first_name.unwrap_or_default(),
            channel_count: c.channels.len(),
            created_at: c.created_at,
        })
        .collect();

    let template = ConversationsNewTemplate {
        contacts: contact_data,
        request_path: "/inbox".to_string(), // Keep them in "Inbox" context
    };

    HtmlTemplate(template).into_response()
}

pub async fn create_ticket(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Form(form): Form<CreateTicketForm>,
) -> impl IntoResponse {
    // Lookup Contact ID via Service (form.contact_id is user_id from the dropdown)
    let contact_id = match crate::services::contact_service::resolve_contact_id_from_user_id(
        &state.db,
        &form.contact_id,
    )
    .await
    {
        Ok(id) => id,
        Err(_) => return Html("Contact not found".to_string()).into_response(),
    };

    // Determine Inbox ID using Service
    let inbox_id = match crate::services::inbox_service::get_default_inbox_id(&state.db).await {
        Ok(id) => id,
        Err(e) => return Html(format!("Error fetching inbox: {}", e)).into_response(),
    };

    let request = crate::models::CreateConversation {
        inbox_id,
        contact_id, // Use resolved INTERNAL contact ID
        subject: Some(form.subject),
    };

    match crate::services::conversation_service::create_conversation(
        &state.db, &auth_user, request, None, // No SLA service for now
    )
    .await
    {
        Ok(conv) => Redirect::to(&format!("/inbox/c/{}", conv.id)).into_response(),
        Err(e) => Html(format!("Error creating ticket: {}", e)).into_response(),
    }
}
