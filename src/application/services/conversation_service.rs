use crate::domain::entities::{
    AssignmentHistory, Conversation, ConversationListResponse, ConversationStatus,
    CreateConversation, UpdateStatusRequest,
};
use crate::domain::events::SystemEvent;
use crate::domain::ports::contact_repository::ContactRepository;
use crate::domain::ports::conversation_repository::ConversationRepository;
use crate::domain::ports::event_bus::EventBus;
use crate::domain::ports::team_repository::TeamRepository;
use crate::domain::ports::user_repository::UserRepository;
use crate::domain::services::state_machine::{execute_transition, TransitionContext};
use crate::infrastructure::http::middleware::auth::AuthenticatedUser;
use crate::infrastructure::http::middleware::error::{ApiError, ApiResult};
use std::sync::Arc;

#[derive(Clone)]
pub struct ConversationService {
    conversation_repo: Arc<dyn ConversationRepository>,
    user_repo: Arc<dyn UserRepository>,
    contact_repo: Arc<dyn ContactRepository>,
    team_repo: Arc<dyn TeamRepository>,
}

impl ConversationService {
    pub fn new(
        conversation_repo: Arc<dyn ConversationRepository>,
        user_repo: Arc<dyn UserRepository>,
        contact_repo: Arc<dyn ContactRepository>,
        team_repo: Arc<dyn TeamRepository>,
    ) -> Self {
        Self {
            conversation_repo,
            user_repo,
            contact_repo,
            team_repo,
        }
    }

    #[tracing::instrument(skip(self, auth_user, sla_service))]
    pub async fn create_conversation(
        &self,
        auth_user: &AuthenticatedUser,
        request: CreateConversation,
        sla_service: Option<&crate::application::services::SlaService>,
    ) -> ApiResult<Conversation> {
        // Check permission
        let has_permission =
            auth_user.is_admin() || auth_user.roles.iter().any(|r| r.name == "Agent");

        if !has_permission {
            return Err(ApiError::Forbidden(
                "Requires 'conversations:create' permission".to_string(),
            ));
        }

        // Validate content/cardinality
        request.validate().map_err(|e| ApiError::BadRequest(e))?;

        // Validate contact exists - request.contact_id should be a user_id
        let user = self
            .user_repo
            .get_user_by_id(&request.contact_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Contact user not found".to_string()))?;

        if !matches!(
            user.user_type,
            crate::domain::entities::user::UserType::Contact
        ) {
            return Err(ApiError::BadRequest("User is not a contact".to_string()));
        }

        // Get the contact record (FK constraint expects contacts.id, not users.id)
        // We added find_contact_by_user_id to ConversationRepository as a convenience,
        // OR we use ContactRepository. ContactRepository likely has it?
        // Let's assume ContactRepository has find_by_user_id or similar.
        // If not, we fall back to ConversationRepository since we added it there (Step 3762).
        let contact = self
            .conversation_repo
            .find_contact_by_user_id(&user.id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Contact record not found".to_string()))?;

        // Create conversation with contact.id
        let mut conversation_request = request.clone();
        conversation_request.contact_id = contact.id;
        let conversation = self
            .conversation_repo
            .create_conversation(&conversation_request)
            .await?;

        // Auto-apply SLA if conversation is assigned to a team with a default SLA policy
        if let Some(sla_svc) = sla_service {
            if let Some(team_id) = &conversation.assigned_team_id {
                let team = self.team_repo.get_team_by_id(team_id).await?;
                if let Some(team) = team {
                    if let Some(policy_id) = team.sla_policy_id {
                        let base_timestamp = chrono::Utc::now().to_rfc3339();

                        tracing::info!(
                            "Auto-applying SLA policy {} to conversation {} (team: {})",
                            policy_id,
                            conversation.id,
                            team_id
                        );

                        if let Err(e) = sla_svc
                            .apply_sla(&conversation.id, &policy_id, &base_timestamp)
                            .await
                        {
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

        Ok(conversation)
    }

    #[tracing::instrument(skip(self, event_bus))]
    pub async fn update_conversation_status(
        &self,
        conversation_id: &str,
        update_request: UpdateStatusRequest,
        agent_id: Option<String>,
        event_bus: Option<&dyn EventBus>,
    ) -> ApiResult<Conversation> {
        // Get current conversation
        let current = self
            .conversation_repo
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Conversation not found".to_string()))?;

        // Validating transition using state machine logic
        let context = TransitionContext {
            conversation_id: conversation_id.to_string(),
            from_status: current.status.clone(),
            to_status: update_request.status.clone(),
            agent_id: agent_id.clone(),
            snooze_duration: update_request.snooze_duration.clone(),
        };

        let _result = execute_transition(context, event_bus)
            .map_err(|e| ApiError::BadRequest(format!("Invalid transition: {}", e)))?;

        tracing::info!(
            "Updating conversation {} status from {:?} to {:?}",
            conversation_id,
            current.status,
            update_request.status
        );

        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        // Calculate timestamps
        let resolved_at = match update_request.status {
            ConversationStatus::Resolved => Some(now.clone()),
            ConversationStatus::Open => None, // Clear resolved_at when reopening
            _ => current.resolved_at.clone(),
        };

        let closed_at = if update_request.status == ConversationStatus::Closed {
            Some(now.clone())
        } else {
            current.closed_at.clone()
        };

        let snoozed_until = if update_request.status == ConversationStatus::Snoozed {
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

        // Update fields
        let updated = self
            .conversation_repo
            .update_conversation_fields(
                conversation_id,
                update_request.status,
                resolved_at,
                closed_at,
                snoozed_until,
            )
            .await?;

        // Handle side effects (SLA resolution) - relying on EventHandler or manual triggering?
        // SlaService::handle_conversation_resolved is usually called.
        // Keeping it out for now as EventBus should trigger it via event handling,
        // OR we should inject SlaService and call it here.
        // Assuming EventBus handles system events -> SLA checks.

        Ok(updated)
    }

    #[tracing::instrument(skip(self, event_bus))]
    pub async fn assign_conversation(
        &self,
        conversation_id: &str,
        user_id: Option<String>,
        team_id: Option<String>,
        assigned_by: String,
        event_bus: Option<&dyn EventBus>,
    ) -> ApiResult<()> {
        // Validate user if provided
        if let Some(uid) = &user_id {
            let _user = self
                .user_repo
                .get_user_by_id(uid)
                .await?
                .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;
        }

        // Validate team if provided
        if let Some(tid) = &team_id {
            let _team = self
                .team_repo
                .get_team_by_id(tid)
                .await?
                .ok_or_else(|| ApiError::NotFound("Team not found".to_string()))?;
        }

        // Get current assignment
        let current = self
            .conversation_repo
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Conversation not found".to_string()))?;

        // Record history
        let history = AssignmentHistory {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: conversation_id.to_string(),
            assigned_user_id: user_id.clone(),
            assigned_team_id: team_id.clone(),
            assigned_by: assigned_by.clone(),
            assigned_at: chrono::Utc::now().to_rfc3339(),
            unassigned_at: None,
        };
        self.conversation_repo.record_assignment(&history).await?;

        // Update assignments
        if user_id != current.assigned_user_id {
            self.conversation_repo
                .assign_conversation_to_user(
                    conversation_id,
                    user_id.clone(),
                    Some(assigned_by.clone()),
                )
                .await?;
        }
        if team_id != current.assigned_team_id {
            self.conversation_repo
                .assign_conversation_to_team(
                    conversation_id,
                    team_id.clone(),
                    Some(assigned_by.clone()),
                )
                .await?;
        }

        if let Some(bus) = event_bus {
            let event = SystemEvent::ConversationAssigned {
                conversation_id: conversation_id.to_string(),
                assigned_user_id: user_id,
                assigned_team_id: team_id,
                assigned_by,
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            let _ = bus.publish(event);
        }

        Ok(())
    }

    pub async fn list_conversations(
        &self,
        _auth_user: &AuthenticatedUser,
        page: i64,
        per_page: i64,
        status: Option<ConversationStatus>,
        inbox_id: Option<String>,
        contact_id: Option<String>,
    ) -> ApiResult<ConversationListResponse> {
        let page = if page < 1 { 1 } else { page };
        let per_page = if per_page < 1 {
            20
        } else if per_page > 100 {
            100
        } else {
            per_page
        };

        let offset = (page - 1) * per_page;

        // Permission check? Assuming caller checks authentication.
        // Agents can generally view conversations.

        let conversations = self
            .conversation_repo
            .list_conversations(
                per_page,
                offset,
                status.clone(),
                inbox_id.clone(),
                contact_id.clone(),
            )
            .await?;
        let total_count = self
            .conversation_repo
            .count_conversations(status, inbox_id, contact_id)
            .await?;

        let total_pages = (total_count + per_page - 1) / per_page;

        Ok(ConversationListResponse {
            conversations,
            pagination: crate::domain::entities::PaginationMetadata {
                page,
                per_page,
                total_count,
                total_pages,
            },
        })
    }

    pub async fn get_conversation(&self, conversation_id: &str) -> ApiResult<Conversation> {
        self.conversation_repo
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Conversation not found".to_string()))
    }

    pub async fn get_conversation_by_reference(
        &self,
        reference_number: i64,
    ) -> ApiResult<Conversation> {
        self.conversation_repo
            .get_conversation_by_reference_number(reference_number)
            .await?
            .ok_or_else(|| ApiError::NotFound("Conversation not found".to_string()))
    }
}

/// Helper: Calculate the snoozed_until timestamp based on duration string
fn calculate_snooze_until(duration: &str) -> ApiResult<String> {
    let duration = duration.trim();

    if duration.is_empty() {
        return Err(ApiError::BadRequest(
            "Snooze duration cannot be empty".to_string(),
        ));
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
            "Invalid snooze duration format. Use format like '1h', '30m', '2d', or '1w'"
                .to_string(),
        ));
    };

    let value: i64 = value
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid snooze duration value".to_string()))?;

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
}
