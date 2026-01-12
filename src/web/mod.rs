use askama::Template;
use axum::{
    extract::{State, Path},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;
use crate::{
    api::middleware::{AppState, AuthenticatedUser},
    models::*,
    services::{self, *},
};

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
}

struct ContactData {
    id: String,
    email: String,
    full_name: String,
    channel_count: usize,
    created_at: String,
}

#[derive(Template)]
#[template(path = "roles.html")]
struct RolesTemplate {
    roles: Vec<RoleData>,
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

// Handlers
pub async fn show_login_page() -> impl IntoResponse {
    let template = LoginTemplate {};
    HtmlTemplate(template)
}

pub async fn handle_login(
    State(state): State<AppState>,
    Form(form): Form<LoginForm>,
) -> Response {
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
            return Html("<div class=\"alert alert-error\">Invalid email or password</div>").into_response();
        }
    };

    tracing::info!("Login successful for user {}", auth_result.user.email);


    // Set session cookie and redirect to dashboard
    (
        StatusCode::OK,
        [
            ("Set-Cookie", format!("session_token={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}", auth_result.session.token, state.session_duration_hours * 3600)),
            ("HX-Redirect", "/dashboard".to_string()),
        ],
    ).into_response()
}

pub async fn show_dashboard(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    // Get stats
    let total_agents = state.db.count_agents().await.unwrap_or(0);
    let total_contacts = state.db.count_contacts().await.unwrap_or(0);
    let roles = state.db.list_roles().await.unwrap_or_default();

    let user_role_names: Vec<String> = auth_user
        .roles
        .iter()
        .map(|r| r.name.clone())
        .collect();

    let template = DashboardTemplate {
        user_name: auth_user.agent.first_name.clone(),
        user_email: auth_user.user.email.clone(),
        user_roles: user_role_names,
        stats: DashboardStats {
            total_agents,
            total_contacts,
            total_roles: roles.len(),
        },
    };

    HtmlTemplate(template)
}

pub async fn handle_logout(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    tracing::info!("Logout for user {} ({})", auth_user.user.email, auth_user.user.id);

    // Delete session
    let _ = state.db.delete_session(&auth_user.token).await;

    // Clear cookie and redirect
    (
        StatusCode::OK,
        [("Set-Cookie", "session_token=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0")],
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
            return Html("<div class=\"alert alert-error\">Failed to load agents</div>").into_response();
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
            id: user.id,  // Use user_id for deletion
            email: user.email,
            first_name: agent.first_name,
            roles: role_names,
            created_at: user.created_at,
        });
    }

    let template = AgentsTemplate {
        agents: agent_data,
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
        }).into_response(),
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
            return Html("<div class=\"alert alert-error\">Failed to load contacts</div>").into_response();
        }
    };

    // Build contact data with channels
    let mut contact_data = Vec::new();
    for (user, contact) in contacts {
        // Get channels for this contact
        let channels = match state.db.get_contact_channels(&contact.id).await {
            Ok(c) => c,
            _ => vec![],
        };

        // Build full name
        let full_name = contact.first_name.unwrap_or_else(|| String::new());

        contact_data.push(ContactData {
            id: user.id,  // Use user_id for deletion
            email: user.email,
            full_name,
            channel_count: channels.len(),
            created_at: user.created_at,
        });
    }

    let template = ContactsTemplate {
        contacts: contact_data,
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
        }).into_response(),
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
            return Html("<div class=\"alert alert-error\">Failed to load roles</div>").into_response();
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
        }).into_response(),
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
