use crate::api::middleware::{ApiResult, ApiError, AuthenticatedUser};
use crate::database::Database;
use crate::models::conversation::*;
use crate::models::user::UserType;
use crate::services::state_machine::{execute_transition, TransitionContext};
use crate::events::EventBus;

/// Update conversation status with validation and event publishing
pub async fn update_conversation_status(
    db: &Database,
    conversation_id: &str,
    update_request: UpdateStatusRequest,
    agent_id: Option<String>,
    event_bus: Option<&EventBus>,
) -> ApiResult<Conversation> {
    // Get current conversation
    let current = db
        .get_conversation_by_id(conversation_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Conversation not found".to_string()))?;

    // Build transition context
    let context = TransitionContext {
        conversation_id: conversation_id.to_string(),
        from_status: current.status,
        to_status: update_request.status,
        agent_id,
        snooze_duration: update_request.snooze_duration.clone(),
    };

    // Execute transition (validates and publishes events)
    let _result = execute_transition(context, event_bus).map_err(|e| {
        ApiError::BadRequest(format!("Invalid transition: {}", e))
    })?;

    tracing::info!(
        "Updating conversation {} status from {:?} to {:?}",
        conversation_id,
        current.status,
        update_request.status
    );

    // Calculate timestamps based on new status
    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap();

    // Feature 019: Handle resolved_at timestamp
    // - Set when transitioning TO Resolved
    // - Clear when reopening (transitioning TO Open from any status)
    let resolved_at = match update_request.status {
        ConversationStatus::Resolved => Some(now.clone()),
        ConversationStatus::Open => None,  // Clear resolved_at when reopening
        _ => current.resolved_at.clone(),  // Preserve existing value for other statuses
    };

    // Feature 019: Set closed_at when transitioning TO Closed
    let closed_at = if update_request.status == ConversationStatus::Closed {
        Some(now.clone())
    } else {
        current.closed_at.clone()  // Preserve existing value
    };

    let snoozed_until = if update_request.status == ConversationStatus::Snoozed {
        // Parse snooze duration and calculate snoozed_until timestamp
        if let Some(duration_str) = &update_request.snooze_duration {
            Some(calculate_snooze_until(duration_str)?)
        } else {
            return Err(ApiError::BadRequest(
                "Snooze duration is required when snoozing a conversation".to_string(),
            ));
        }
    } else {
        None
    };

    // Update the conversation in database
    let updated = db
        .update_conversation_fields(conversation_id, update_request.status, resolved_at, closed_at, snoozed_until)
        .await?;

    tracing::info!(
        "Conversation {} status updated successfully to {:?}",
        conversation_id,
        updated.status
    );

    Ok(updated)
}

/// Calculate the snoozed_until timestamp based on duration string
/// Supports formats like "1h", "30m", "2d", "1w"
fn calculate_snooze_until(duration: &str) -> ApiResult<String> {
    let duration = duration.trim();

    if duration.is_empty() {
        return Err(ApiError::BadRequest("Snooze duration cannot be empty".to_string()));
    }

    // Parse the duration
    let (value, unit) = if let Some(stripped) = duration.strip_suffix('h') {
        (stripped, 'h')
    } else if let Some(stripped) = duration.strip_suffix('m') {
        (stripped, 'm')
    } else if let Some(stripped) = duration.strip_suffix('d') {
        (stripped, 'd')
    } else if let Some(stripped) = duration.strip_suffix('w') {
        (stripped, 'w')
    } else {
        return Err(ApiError::BadRequest(
            "Invalid snooze duration format. Use format like '1h', '30m', '2d', or '1w'".to_string(),
        ));
    };

    let value: i64 = value.parse().map_err(|_| {
        ApiError::BadRequest("Invalid snooze duration value".to_string())
    })?;

    if value <= 0 {
        return Err(ApiError::BadRequest(
            "Snooze duration must be positive".to_string(),
        ));
    }

    // Calculate offset in seconds
    let seconds = match unit {
        'm' => value * 60,
        'h' => value * 3600,
        'd' => value * 86400,
        'w' => value * 604800,
        _ => return Err(ApiError::BadRequest("Invalid time unit".to_string())),
    };

    // Add offset to current time
    let now = time::OffsetDateTime::now_utc();
    let duration = time::Duration::seconds(seconds);
    let snoozed_until = now + duration;

    Ok(snoozed_until
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap())
}

/// Create a new conversation
pub async fn create_conversation(
    db: &Database,
    auth_user: &AuthenticatedUser,
    request: CreateConversation,
    sla_service: Option<&crate::services::SlaService>,
) -> ApiResult<Conversation> {
    // Check permission
    let has_permission = auth_user.is_admin()
        || auth_user.roles.iter().any(|r| r.name == "Agent");

    if !has_permission {
        return Err(ApiError::Forbidden(
            "Requires 'conversations:create' permission".to_string(),
        ));
    }

    // TODO: Validate inbox exists when inbox management is implemented
    // let _inbox = db.get_inbox_by_id(&request.inbox_id).await?
    //     .ok_or_else(|| ApiError::NotFound("Inbox not found".to_string()))?;

    // Validate contact exists
    let contact = db
        .get_user_by_id(&request.contact_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Contact not found".to_string()))?;

    if !matches!(contact.user_type, UserType::Contact) {
        return Err(ApiError::BadRequest("User is not a contact".to_string()));
    }

    // Create conversation
    let conversation = db.create_conversation(&request).await?;

    // Auto-apply SLA if conversation is assigned to a team with a default SLA policy
    if let Some(sla_svc) = sla_service {
        if let Some(team_id) = &conversation.assigned_team_id {
            if let Ok(Some(team)) = db.get_team_by_id(team_id).await {
                if let Some(policy_id) = team.sla_policy_id {
                    let base_timestamp = chrono::Utc::now().to_rfc3339();

                    tracing::info!(
                        "Auto-applying SLA policy {} to conversation {} (team: {})",
                        policy_id,
                        conversation.id,
                        team_id
                    );

                    match sla_svc.apply_sla(&conversation.id, &policy_id, &base_timestamp).await {
                        Ok(_) => {
                            tracing::info!(
                                "Successfully auto-applied SLA policy {} to conversation {}",
                                policy_id,
                                conversation.id
                            );
                        }
                        Err(e) => {
                            // Log error but don't fail conversation creation
                            tracing::error!(
                                "Failed to auto-apply SLA policy {} to conversation {}: {}",
                                policy_id,
                                conversation.id,
                                e
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(conversation)
}

/// Get conversation by ID
pub async fn get_conversation(
    db: &Database,
    conversation_id: &str,
) -> ApiResult<Conversation> {
    db.get_conversation_by_id(conversation_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Conversation not found".to_string()))
}

/// Get conversation by reference number
pub async fn get_conversation_by_reference(
    db: &Database,
    reference_number: i64,
) -> ApiResult<Conversation> {
    db.get_conversation_by_reference(reference_number)
        .await?
        .ok_or_else(|| ApiError::NotFound("Conversation not found".to_string()))
}

/// List conversations with pagination and filters
pub async fn list_conversations(
    db: &Database,
    page: i64,
    per_page: i64,
    status: Option<ConversationStatus>,
    inbox_id: Option<String>,
    contact_id: Option<String>,
) -> ApiResult<ConversationListResponse> {
    // Validate pagination
    let page = if page < 1 { 1 } else { page };
    let per_page = if per_page < 1 {
        20
    } else if per_page > 100 {
        100
    } else {
        per_page
    };

    let offset = (page - 1) * per_page;

    // Get conversations
    let conversations = db
        .list_conversations(per_page, offset, status, inbox_id.clone(), contact_id.clone())
        .await?;

    // Get total count
    let total_count = db
        .count_conversations(status, inbox_id, contact_id)
        .await?;

    let total_pages = (total_count + per_page - 1) / per_page;

    Ok(ConversationListResponse {
        conversations,
        pagination: crate::models::PaginationMetadata {
            page,
            per_page,
            total_count,
            total_pages,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_snooze_until_valid_minutes() {
        let result = calculate_snooze_until("30m");
        assert!(result.is_ok());
    }

    #[test]
    fn test_calculate_snooze_until_valid_hours() {
        let result = calculate_snooze_until("2h");
        assert!(result.is_ok());
    }

    #[test]
    fn test_calculate_snooze_until_valid_days() {
        let result = calculate_snooze_until("3d");
        assert!(result.is_ok());
    }

    #[test]
    fn test_calculate_snooze_until_valid_weeks() {
        let result = calculate_snooze_until("1w");
        assert!(result.is_ok());
    }

    #[test]
    fn test_calculate_snooze_until_invalid_format() {
        let result = calculate_snooze_until("30");
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_snooze_until_invalid_value() {
        let result = calculate_snooze_until("abch");
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_snooze_until_negative_value() {
        let result = calculate_snooze_until("-5h");
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_snooze_until_zero_value() {
        let result = calculate_snooze_until("0h");
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_snooze_until_empty() {
        let result = calculate_snooze_until("");
        assert!(result.is_err());
    }
}
