use crate::{
    domain::entities::CreateAgentRequest,
    infrastructure::http::middleware::{AppState, AuthenticatedUser},
};
use askama::Template;
use axum::{
    extract::{Path, Query, State},
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
    is_admin: bool,
}

struct DashboardStats {
    total_agents: i64,
    total_contacts: i64,
    total_roles: usize,
}

#[derive(Template)]
#[template(path = "agent_dashboard.html")]
struct AgentDashboardTemplate {
    agent_name: String,
    my_open_count: i64,
    unassigned_count: i64,
    sla_at_risk_count: i64,
    resolved_today_count: i64,
    my_conversations: Vec<DashboardConversationData>,
    unassigned_conversations: Vec<DashboardConversationData>,
    recent_activity: Vec<ActivityData>,
    request_path: String,
    is_admin: bool,
}

struct DashboardConversationData {
    id: String,
    subject: String,
    contact_name: String,
    status: String,
    updated_at: String,
    created_at: String,
    tags: Vec<TagData>,
    has_sla: bool,
    sla_status: String,
    sla_time_remaining: String,
    unread_count: i64,
}

struct ActivityData {
    r#type: String,
    description: String,
    time_ago: String,
}

#[derive(Template)]
#[template(path = "agents.html")]
struct AgentsTemplate {
    agents: Vec<AgentData>,
    roles: Vec<RoleData>,
    request_path: String,
    is_admin: bool,
}

#[derive(Template)]
#[template(path = "agents_new.html")]
struct AgentsNewTemplate {
    roles: Vec<RoleData>,
    request_path: String,
    is_admin: bool,
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
    is_admin: bool,
}

#[derive(Template)]
#[template(path = "contacts_new.html")]
struct ContactsNewTemplate {
    request_path: String,
    is_admin: bool,
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
    is_admin: bool,
}

#[derive(Template)]
#[template(path = "conversations_new.html")]
struct ConversationsNewTemplate {
    contacts: Vec<ContactData>,
    request_path: String,
    is_admin: bool,
}

#[derive(Template)]
#[template(path = "roles.html")]
struct RolesTemplate {
    roles: Vec<RoleData>,
    request_path: String,
    is_admin: bool,
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
    all_tags: Vec<TagData>,
    all_tags_json: String,
    request_path: String,
    is_admin: bool,
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
    all_tags: Vec<TagData>,
    all_tags_json: String,
}

#[derive(Template)]
#[template(path = "public_conversation.html")]
struct PublicConversationTemplate {
    conversation: ConversationDetailData,
    messages: Vec<MessageData>,
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
    tags: Vec<TagData>,
    tags_json: String,
    applied_sla: Option<AppliedSlaData>,
    applied_sla_json: String,
}

#[derive(serde::Serialize, Clone)]
struct TagData {
    id: String,
    name: String,
    color: String,
}

#[derive(serde::Serialize)]
struct AppliedSlaData {
    policy_name: String,
    first_response_deadline: String,
    response_deadline_timestamp: i64,
    resolution_deadline: String,
    resolution_deadline_timestamp: i64,
    status: String,
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
    is_admin: bool,
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

#[derive(Template)]
#[template(path = "agent_created.html")]
struct AgentCreatedTemplate {
    agent_name: String,
    agent_email: String,
    agent_password: String,
    agent_role: String,
    request_path: String,
    is_admin: bool,
}

// Temporarily commented out while debugging template issues
// #[derive(Template)]
// #[template(path = "teams.html")]
struct TeamsTemplate {
    teams: Vec<TeamData>,
    has_teams: bool,
    request_path: String,
    is_admin: bool,
}

struct TeamData {
    id: String,
    name: String,
    description: String,
    members: Vec<TeamMemberData>,
}

struct TeamMemberData {
    first_name: String,
    first_name_initial: String,
    last_name: Option<String>,
    email: String,
    availability: String,
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

#[derive(Deserialize)]
pub struct InboxFilterParams {
    view: Option<String>,
}

// Handlers
pub async fn show_login_page() -> impl IntoResponse {
    let template = LoginTemplate {};
    HtmlTemplate(template)
}

pub async fn handle_login(State(state): State<AppState>, Form(form): Form<LoginForm>) -> Response {
    // Delegate to auth service
    let auth_result = match state
        .auth_service
        .authenticate(&form.email, &form.password, state.session_duration_hours)
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
) -> Response {
    // Route to appropriate dashboard based on role
    if auth_user.is_admin() {
        show_admin_dashboard(State(state), axum::Extension(auth_user)).await.into_response()
    } else {
        show_agent_dashboard(State(state), axum::Extension(auth_user)).await.into_response()
    }
}

pub async fn show_admin_dashboard(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    // Get stats
    let total_agents = state.agent_service.count_agents().await.unwrap_or(0);
    let total_contacts = state.contact_service.count_contacts().await.unwrap_or(0);
    let roles = state
        .role_service
        .list_roles_raw()
        .await
        .unwrap_or_default();

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
        is_admin: auth_user.is_admin(),
    };

    HtmlTemplate(template).into_response()
}

pub async fn show_agent_dashboard(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    let agent_name = format!(
        "{} {}",
        auth_user.agent.first_name,
        auth_user.agent.last_name.as_deref().unwrap_or("")
    );

    // Fetch my assigned conversations
    let (my_conversations_raw, _my_open_count) = state
        .assignment_service
        .get_user_assigned_conversations(&auth_user.user.id, 100, 0)
        .await
        .unwrap_or((vec![], 0));

    // Filter for only open conversations
    use crate::domain::entities::ConversationStatus;
    let my_open_conversations: Vec<_> = my_conversations_raw
        .iter()
        .filter(|c| c.status != ConversationStatus::Resolved && c.status != ConversationStatus::Closed)
        .cloned()
        .collect();

    let actual_my_open_count = my_open_conversations.len() as i64;

    // Fetch unassigned conversations
    let (unassigned_conversations_raw, unassigned_count) = state
        .assignment_service
        .get_unassigned_conversations(20, 0)
        .await
        .unwrap_or((vec![], 0));

    // TODO: Calculate SLA at risk count by fetching applied SLAs
    let sla_at_risk_count = 0;

    // TODO: Calculate resolved today count by filtering conversations
    let resolved_today_count = 0;

    // Convert conversations to dashboard data
    let my_conversations = futures::future::join_all(
        my_open_conversations
            .iter()
            .take(10)
            .map(|c| conversation_to_dashboard_data(c.clone(), &state)),
    )
    .await;

    let unassigned_conversations = futures::future::join_all(
        unassigned_conversations_raw
            .iter()
            .take(10)
            .map(|c| conversation_to_dashboard_data(c.clone(), &state)),
    )
    .await;

    // TODO: Fetch recent activity (recent messages, assignments, status changes)
    let recent_activity = vec![];

    let template = AgentDashboardTemplate {
        agent_name,
        my_open_count: actual_my_open_count,
        unassigned_count,
        sla_at_risk_count,
        resolved_today_count,
        my_conversations,
        unassigned_conversations,
        recent_activity,
        request_path: "/dashboard".to_string(),
        is_admin: auth_user.is_admin(),
    };

    HtmlTemplate(template).into_response()
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
    let _ = state.session_service.delete_session(&auth_user.token).await;

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
    let agents = match state.agent_service.list_agents_raw(1000, 0).await {
        Ok(agents) => agents,
        Err(_) => {
            return Html("<div class=\"alert alert-error\">Failed to load agents</div>")
                .into_response();
        }
    };

    // Build agent data with roles
    let mut agent_data = Vec::new();
    for (user, agent) in agents {
        let roles = match state.role_service.get_user_roles(&user.id).await {
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
    let all_roles = match state.role_service.list_roles_raw().await {
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
        is_admin: _auth_user.is_admin(),
    };

    HtmlTemplate(template).into_response()
}

pub async fn delete_agent(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Response {
    match state.agent_service.delete(&auth_user, &id).await {
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
        email: form.email.clone(),
        first_name: form.first_name.clone(),
        last_name: form.last_name.clone(),
        role_id: Some(form.role_id.clone()),
    };

    match state.agent_service.create_agent(&auth_user, request).await {
        Ok(response) => {
            // Get role name for display
            let role_name = match state.role_service.get_role(&form.role_id).await {
                Ok(role) => role.name,
                _ => "Agent".to_string(),
            };

            // Display the success page with the password (shown only once)
            let full_name = if let Some(last) = &form.last_name {
                format!("{} {}", form.first_name, last)
            } else {
                form.first_name.clone()
            };

            let template = AgentCreatedTemplate {
                agent_name: full_name,
                agent_email: form.email,
                agent_password: response.password,
                agent_role: role_name,
                request_path: "/agents".to_string(),
                is_admin: auth_user.is_admin(),
            };

            HtmlTemplate(template).into_response()
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
    // Get all contacts
    let contacts = match state.contact_service.list_contacts(1, 1000).await {
        Ok(list) => list.contacts,
        Err(_) => {
            return Html("<div class=\"alert alert-error\">Failed to load contacts</div>")
                .into_response();
        }
    };

    // Build contact data
    let contact_data: Vec<ContactData> = contacts
        .into_iter()
        .map(|c| ContactData {
            id: c.id,
            email: c.email,
            full_name: c.first_name.unwrap_or_default(),
            channel_count: c.channels.len(),
            created_at: c.created_at,
        })
        .collect();

    let template = ContactsTemplate {
        contacts: contact_data,
        request_path: "/contacts".to_string(),
        is_admin: _auth_user.is_admin(),
    };

    HtmlTemplate(template).into_response()
}

pub async fn delete_contact(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Response {
    match state.contact_service.delete(&auth_user, &id).await {
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
    let roles = match state.role_service.list_roles().await {
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
        let permissions = match state.role_service.get_role_permissions(&role.id).await {
            Ok(perms) => perms,
            _ => vec![],
        };

        let permission_names: Vec<String> = permissions.iter().map(|p| p.name.clone()).collect();

        // Get user count
        let user_count = match state.role_service.count_users_with_role(&role.id).await {
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
        is_admin: _auth_user.is_admin(),
    };

    HtmlTemplate(template).into_response()
}

pub async fn delete_role(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Response {
    match state.role_service.delete(&auth_user.user, &id).await {
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
    let all_roles = match state.role_service.list_roles_raw().await {
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
        is_admin: _auth_user.is_admin(),
    };

    HtmlTemplate(template).into_response()
}

pub async fn show_create_contact_page(
    State(_state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    let template = ContactsNewTemplate {
        request_path: "/contacts".to_string(), // Keep 'Contacts' active in sidebar
        is_admin: auth_user.is_admin(),
    };

    HtmlTemplate(template).into_response()
}

pub async fn create_contact(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Form(form): Form<CreateContactForm>,
) -> Response {
    let request = crate::domain::entities::CreateContactRequest {
        email: form.email,
        first_name: form.full_name,
        inbox_id: String::new(), // No inbox selected for basic contact creation
    };

    match state
        .contact_service
        .create_contact(&auth_user, request)
        .await
    {
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
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Query(params): Query<InboxFilterParams>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let is_htmx = headers.get("HX-Request").is_some();
    let view = params.view.as_deref().unwrap_or("all");

    // Fetch all conversations first
    let all_conversations = match state
        .conversation_service
        .list_conversations(&auth_user, 1, 100, None, None, None)
        .await
    {
        Ok(list) => list.conversations,
        Err(_) => vec![],
    };

    // Filter based on view
    let conversations: Vec<_> = match view {
        "unassigned" => {
            // Filter for unassigned conversations
            all_conversations
                .into_iter()
                .filter(|conv| conv.assigned_user_id.is_none() && conv.assigned_team_id.is_none())
                .collect()
        },
        "mine" => {
            // Filter for conversations assigned to current user
            let user_id = auth_user.user.id.clone();
            all_conversations
                .into_iter()
                .filter(move |conv| {
                    if let Some(assigned_id) = &conv.assigned_user_id {
                        assigned_id == &user_id
                    } else {
                        false
                    }
                })
                .collect()
        },
        "team" => {
            // Filter for conversations assigned to user's teams
            // Get user's teams first
            let user_teams = match state.team_service.get_user_teams(&auth_user.user.id).await {
                Ok(teams) => teams,
                Err(_) => vec![],
            };

            let team_ids: Vec<String> = user_teams.into_iter().map(|t| t.id).collect();

            all_conversations
                .into_iter()
                .filter(move |conv| {
                    if let Some(assigned_team) = &conv.assigned_team_id {
                        team_ids.contains(assigned_team)
                    } else {
                        false
                    }
                })
                .collect()
        },
        _ => {
            // Default: all conversations
            all_conversations
        }
    };

    let mut conversation_data = Vec::new();
    for conv in conversations {
        let contact_name = match state.user_service.get_user_by_id(&conv.contact_id).await {
            Ok(Some(u)) => match state.contact_service.find_contact_by_user_id(&u.id).await {
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

    // If HTMX request, return just the conversation list partial
    if is_htmx {
        let template = ConversationListPartial {
            conversations: conversation_data,
            selected_id: None,
        };
        return HtmlTemplate(template).into_response();
    }

    // Otherwise, return full page
    let dummy_conversation = ConversationDetailData {
        id: String::new(),
        subject: String::new(),
        contact_name: String::new(),
        status: String::new(),
        assigned_user_id: None,
        tags: vec![],
        tags_json: "[]".to_string(),
        applied_sla: None,
        applied_sla_json: "null".to_string(),
    };

    let template = InboxTemplate {
        conversations: conversation_data,
        selected_id: None,
        conversation: dummy_conversation,
        messages: vec![],
        agents: vec![],
        all_tags: vec![],
        all_tags_json: "[]".to_string(),
        request_path: "/inbox".to_string(),
        is_admin: auth_user.is_admin(),
    };

    HtmlTemplate(template).into_response()
}

pub async fn show_conversation(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let is_htmx = headers.get("HX-Request").is_some();

    // Fetch conversation
    let conversation = match state.conversation_service.get_conversation(&id).await {
        Ok(c) => c,
        Err(_) => return Html("Conversation not found").into_response(),
    };

    // Fetch contact name for detail view
    let contact_name = match state
        .user_service
        .get_user_by_id(&conversation.contact_id)
        .await
    {
        Ok(Some(u)) => match state.contact_service.find_contact_by_user_id(&u.id).await {
            Ok(Some(c)) => c.first_name.unwrap_or_else(|| u.email.clone()),
            _ => u.email,
        },
        _ => "Unknown".to_string(),
    };

    // Fetch conversation tags
    let conversation_tags = match state.conversation_tag_service.get_conversation_tags(&id).await {
        Ok(tags) => tags
            .into_iter()
            .map(|t| TagData {
                id: t.id,
                name: t.name,
                color: t.color.unwrap_or_else(|| "#06b6d4".to_string()),
            })
            .collect(),
        Err(_) => vec![],
    };

    let tags_json = serde_json::to_string(&conversation_tags).unwrap_or_else(|_| "[]".to_string());

    // Fetch applied SLA
    let (applied_sla, applied_sla_json) = match state.sla_service.get_applied_sla_by_conversation(&id).await {
        Ok(Some(sla)) => {
            // Parse timestamps from datetime strings
            // For simplicity, we'll use a mock timestamp calculation
            // In production, use chrono to properly parse and convert
            let sla_data = AppliedSlaData {
                policy_name: "SLA Policy".to_string(), // TODO: Fetch policy name from sla_policy_id
                first_response_deadline: format_datetime(&sla.first_response_deadline_at),
                response_deadline_timestamp: 0, // TODO: Parse from datetime string
                resolution_deadline: format_datetime(&sla.resolution_deadline_at),
                resolution_deadline_timestamp: 0, // TODO: Parse from datetime string
                status: sla.status.to_string(),
            };
            let json = serde_json::to_string(&sla_data).unwrap_or_else(|_| "null".to_string());
            (Some(sla_data), json)
        }
        _ => (None, "null".to_string()),
    };

    // Prepare Detail Data
    let detail_data = ConversationDetailData {
        id: conversation.id.clone(),
        subject: conversation.subject.clone().unwrap_or_default(),
        contact_name: contact_name.clone(),
        status: conversation.status.to_string(),
        assigned_user_id: conversation.assigned_user_id.clone(),
        tags: conversation_tags,
        tags_json,
        applied_sla,
        applied_sla_json,
    };

    // Fetch Agents for Dropdown
    let agents_list = match state.agent_service.list_agents_raw(1000, 0).await {
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

    // Fetch all tags for tag picker
    // Get actual Permission objects from roles
    let mut permissions = Vec::new();
    for role in &auth_user.roles {
        if let Ok(role_perms) = state.role_service.get_role_permissions(&role.id).await {
            permissions.extend(role_perms);
        }
    }

    let all_tags = match state.tag_service.list_tags(100, 0, &permissions).await {
        Ok((tags, _total)) => tags
            .into_iter()
            .map(|t| TagData {
                id: t.id,
                name: t.name,
                color: t.color.unwrap_or_else(|| "#06b6d4".to_string()),
            })
            .collect(),
        Err(_) => vec![],
    };

    let all_tags_json = serde_json::to_string(&all_tags).unwrap_or_else(|_| "[]".to_string());

    // Message Logic
    let (messages, _total) = match state.message_service.list_messages(&id, 1, 50).await {
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
            all_tags: all_tags.clone(),
            all_tags_json: all_tags_json.clone(),
        };
        HtmlTemplate(template).into_response()
    } else {
        // Full page render
        let all_convs = match state
            .conversation_service
            .list_conversations(&auth_user, 1, 50, None, None, None)
            .await
        {
            Ok(list) => list.conversations,
            Err(_) => vec![],
        };

        let mut conversation_data = Vec::new();
        for conv in all_convs {
            let c_name = match state.user_service.get_user_by_id(&conv.contact_id).await {
                Ok(Some(u)) => match state.contact_service.find_contact_by_user_id(&u.id).await {
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
            all_tags,
            all_tags_json,
            request_path: "/inbox".to_string(),
            is_admin: auth_user.is_admin(),
        };
        HtmlTemplate(template).into_response()
    }
}

/*  // Temporarily commented out while debugging template issues
pub async fn show_teams(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    // Fetch user's teams
    let user_teams = match state.team_service.get_user_teams(&auth_user.user.id).await {
        Ok(teams) => teams,
        Err(_) => vec![],
    };

    let mut team_data = Vec::new();

    for team in user_teams {
        // Fetch team members
        let members = match state.team_service.get_members(&team.id).await {
            Ok(members) => members,
            Err(_) => vec![],
        };

        let mut member_data = Vec::new();
        for user in members {
            // Fetch agent data for this user
            let agent = match state.agent_service.get_agent_by_user_id(&user.id).await {
                Ok(Some(a)) => a,
                _ => continue, // Skip if not an agent
            };

            // Fetch availability status for each member
            let availability = match state
                .availability_service
                .get_availability(&user.id)
                .await
            {
                Ok(status) => status.availability_status.to_string(),
                Err(_) => "offline".to_string(),
            };

            let first_name_initial = agent.first_name.chars().next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_else(|| "?".to_string());

            member_data.push(TeamMemberData {
                first_name: agent.first_name,
                first_name_initial,
                last_name: agent.last_name,
                email: user.email,
                availability,
            });
        }

        team_data.push(TeamData {
            id: team.id,
            name: team.name,
            description: team.description.unwrap_or_default(),
            members: member_data,
        });
    }

    let has_teams = !team_data.is_empty();

    let template = TeamsTemplate {
        teams: team_data,
        has_teams,
        request_path: "/teams".to_string(),
        is_admin: auth_user.is_admin(),
    };

    HtmlTemplate(template).into_response()
}
*/  // End of temporarily commented out show_teams

// Helper function to format datetime for display
fn format_datetime(datetime_str: &str) -> String {
    // For now, just return a truncated version
    // In production, use chrono to parse and format properly
    datetime_str.chars().take(16).collect()
}

// Helper function to calculate time ago from timestamp
fn time_ago(_datetime_str: &str) -> String {
    // Simplified implementation - in production use chrono
    // For now, just return "recently"
    "recently".to_string()
}

// Helper function to convert Conversation to DashboardConversationData
async fn conversation_to_dashboard_data(
    conversation: crate::domain::entities::Conversation,
    state: &AppState,
) -> DashboardConversationData {
    use crate::domain::entities::AppliedSlaStatus;

    // Fetch contact name
    let contact_name = if let Ok(contact) = state.contact_service.get_contact(&conversation.contact_id).await {
        contact.first_name.unwrap_or_else(|| "Unknown".to_string())
    } else {
        "Unknown".to_string()
    };

    // Fetch conversation tags
    let tags = match state.conversation_tag_service.get_conversation_tags(&conversation.id).await {
        Ok(tags) => tags
            .into_iter()
            .map(|t| TagData {
                id: t.id.clone(),
                name: t.name.clone(),
                color: t.color.unwrap_or_else(|| "#6B7280".to_string()),
            })
            .collect(),
        Err(_) => vec![],
    };

    // Fetch applied SLA
    let (has_sla, sla_status, sla_time_remaining) =
        match state.sla_service.get_applied_sla_by_conversation(&conversation.id).await {
            Ok(Some(sla)) => {
                let status = if sla.status == AppliedSlaStatus::Breached {
                    "breached"
                } else if sla.status == AppliedSlaStatus::Pending {
                    "warning"
                } else {
                    "ok"
                };
                // Simple time remaining calculation
                let time_remaining = "2h 30m".to_string(); // TODO: Calculate from deadline
                (true, status.to_string(), time_remaining)
            }
            _ => (false, "".to_string(), "".to_string()),
        };

    // TODO: Calculate unread count from messages
    let unread_count = 0;

    DashboardConversationData {
        id: conversation.id.clone(),
        subject: conversation.subject.unwrap_or_else(|| "No subject".to_string()),
        contact_name,
        status: conversation.status.to_string(),
        updated_at: format_datetime(&conversation.updated_at),
        created_at: format_datetime(&conversation.created_at),
        tags,
        has_sla,
        sla_status,
        sla_time_remaining,
        unread_count,
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

    match state
        .conversation_service
        .assign_conversation(
            &id,
            Some(form.agent_id.clone()), // Assign to user
            None,                        // Assign to team (none)
            auth_user.user.id.clone(),   // assigned_by
            None,                        // event_bus
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
    let request = crate::domain::entities::conversation::UpdateStatusRequest {
        status: crate::domain::entities::conversation::ConversationStatus::Resolved,
        snooze_duration: None,
    };

    match state
        .conversation_service
        .update_conversation_status(
            &id, request, None, None, // No event bus for now or pass context
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
    let request = crate::domain::entities::conversation::UpdateStatusRequest {
        status: crate::domain::entities::conversation::ConversationStatus::Open,
        snooze_duration: None,
    };

    match state
        .conversation_service
        .update_conversation_status(&id, request, None, None)
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
    let request = crate::domain::entities::SendMessageRequest {
        content: form.content,
    };

    match state.message_service.send_message(id.clone(), auth_user.user.id, request).await {
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
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>, // This is the user_id (since contacts list uses user.id)
) -> impl IntoResponse {
    // 1. Fetch User (to get email and created_at)
    let user = match state.user_service.get_user_by_id(&id).await {
        Ok(Some(u)) => u,
        Ok(None) => return Html("User not found").into_response(),
        Err(_) => return Html("Error fetching user").into_response(),
    };

    // 2. Fetch Contact (to get Name)
    let contact = match state.contact_service.find_contact_by_user_id(&id).await {
        Ok(Some(c)) => c,
        Ok(None) => return Html("Contact not found").into_response(), // Should theoretically exist if listed
        Err(_) => return Html("Error fetching contact").into_response(),
    };

    // 3. Fetch Channels
    let channels = match state
        .contact_service
        .find_contact_channels(&contact.id)
        .await
    {
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

    let conversations = match state
        .conversation_service
        .list_conversations(
            &auth_user,
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
        is_admin: auth_user.is_admin(),
    };

    HtmlTemplate(template).into_response()
}

pub async fn show_contact_edit(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let user = match state.user_service.get_user_by_id(&id).await {
        Ok(Some(u)) => u,
        Ok(None) => return Html("User not found").into_response(),
        Err(_) => return Html("Error fetching user").into_response(),
    };

    let contact = match state.contact_service.find_contact_by_user_id(&id).await {
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
        is_admin: _auth_user.is_admin(),
    };

    HtmlTemplate(template).into_response()
}

pub async fn update_contact(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Form(form): Form<ContactUpdateForm>,
) -> impl IntoResponse {
    if let Err(e) = state
        .contact_service
        .update_contact_details(&auth_user, &id, &form.full_name, &form.email)
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
    let contacts = match state
        .contact_service
        .list_contacts(
            1, 1000, // reasonable limit for dropdown
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
        is_admin: _auth_user.is_admin(),
    };

    HtmlTemplate(template).into_response()
}

pub async fn create_ticket(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Form(form): Form<CreateTicketForm>,
) -> impl IntoResponse {
    // Lookup Contact ID via Service (form.contact_id is user_id from the dropdown)
    let contact_id = match state
        .contact_service
        .resolve_contact_id_from_user_id(&form.contact_id)
        .await
    {
        Ok(id) => id,
        Err(_) => return Html("Contact not found".to_string()).into_response(),
    };

    // Determine Inbox ID using Service
    let inbox_id = match state.inbox_service.get_default_inbox_id().await {
        Ok(id) => id,
        Err(e) => return Html(format!("Error fetching inbox: {}", e)).into_response(),
    };

    let request = crate::domain::entities::CreateConversation {
        inbox_id,
        contact_id, // Use resolved INTERNAL contact ID
        subject: Some(form.subject),
    };

    match state
        .conversation_service
        .create_conversation(
            &auth_user, request, None, // No SLA service for now
        )
        .await
    {
        Ok(conv) => Redirect::to(&format!("/inbox/c/{}", conv.id)).into_response(),
        Err(e) => Html(format!("Error creating ticket: {}", e)).into_response(),
    }
}

pub async fn show_public_conversation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // 1. Fetch conversation (Publicly accessible)
    let conversation = match state.conversation_service.get_conversation(&id).await {
        Ok(c) => c,
        Err(_) => return Html("Conversation not found").into_response(),
    };

    // 2. Fetch contact name
    let contact_name = match state
        .user_service
        .get_user_by_id(&conversation.contact_id)
        .await
    {
        Ok(Some(u)) => match state.contact_service.find_contact_by_user_id(&u.id).await {
            Ok(Some(c)) => c.first_name.unwrap_or_else(|| u.email.clone()),
            _ => u.email,
        },
        _ => "Customer".to_string(),
    };

    let detail_data = ConversationDetailData {
        id: conversation.id.clone(),
        subject: conversation.subject.clone().unwrap_or_default(),
        contact_name,
        status: conversation.status.to_string(),
        assigned_user_id: None, // Not needed for public view
        tags: vec![],
        tags_json: "[]".to_string(),
        applied_sla: None,
        applied_sla_json: "null".to_string(),
    };

    // 3. Fetch messages (Publicly accessible)
    let (messages, _total) = match state.message_service.list_messages(&id, 1, 100).await {
        Ok(res) => res,
        Err(_) => (vec![], 0),
    };

    let mut message_data = Vec::new();
    for msg in messages {
        let (sender_name, is_agent) = if msg.author_id == conversation.contact_id {
            ("You".to_string(), false)
        } else {
            ("Oxidesk Support".to_string(), true)
        };

        message_data.push(MessageData {
            id: msg.id,
            sender_name,
            content: msg.content,
            is_agent,
            created_at: msg.created_at,
        });
    }

    let template = PublicConversationTemplate {
        conversation: detail_data,
        messages: message_data,
    };

    HtmlTemplate(template).into_response()
}
