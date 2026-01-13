use crate::{
    api::middleware::error::{ApiError, ApiResult},
    database::Database,
    events::{EventBus, SystemEvent},
    models::{AgentAvailability, AssignmentHistory, Conversation, ConversationStatus, Permission},
    services::notification_service::NotificationService,
};

pub struct AssignmentService {
    db: Database,
    event_bus: EventBus,
    notification_service: NotificationService,
}

impl AssignmentService {
    pub fn new(
        db: Database,
        event_bus: EventBus,
        notification_service: NotificationService,
    ) -> Self {
        Self {
            db,
            event_bus,
            notification_service,
        }
    }

    // Helper: Check if user has permission
    fn has_permission(&self, permissions: &[Permission], required: &str) -> bool {
        permissions.iter().any(|p| p.name == required)
    }

    // User Story 1: Self-assignment
    pub async fn self_assign_conversation(
        &self,
        conversation_id: &str,
        agent_id: &str,
        permissions: &[Permission],
    ) -> ApiResult<Conversation> {
        // 1. Check permission
        if !self.has_permission(permissions, "conversations:update_user_assignee") {
            return Err(ApiError::Forbidden(
                "Missing permission: conversations:update_user_assignee".to_string(),
            ));
        }

        // 2. Verify conversation exists
        let conversation = self
            .db
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Conversation {} not found", conversation_id))
            })?;

        // 3. Assign to database
        self.db
            .assign_conversation_to_user(conversation_id, agent_id, agent_id)
            .await?;

        // 4. Add as participant (ignore if already exists)
        let _ = self
            .db
            .add_conversation_participant(conversation_id, agent_id, Some(agent_id))
            .await;

        // 5. Record in history
        let history = AssignmentHistory::new(
            conversation_id.to_string(),
            Some(agent_id.to_string()),
            None,
            agent_id.to_string(),
        );
        self.db.record_assignment(&history).await?;

        // 6. Publish event
        self.event_bus.publish(SystemEvent::ConversationAssigned {
            conversation_id: conversation_id.to_string(),
            assigned_user_id: Some(agent_id.to_string()),
            assigned_team_id: None,
            assigned_by: agent_id.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        // 7. Send notification (async, fire-and-forget)
        let notification_service = self.notification_service.clone();
        let user_id = agent_id.to_string();
        let conv_id = conversation_id.to_string();
        tokio::spawn(async move {
            let _ = notification_service.notify_assignment(&user_id, &conv_id).await;
        });

        // 8. Return updated conversation
        self.db
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::Internal("Conversation disappeared after assignment".to_string())
            })
    }

    // User Story 2: Agent-to-agent assignment
    pub async fn assign_conversation_to_agent(
        &self,
        conversation_id: &str,
        target_agent_id: &str,
        assigning_agent_id: &str,
        permissions: &[Permission],
    ) -> ApiResult<Conversation> {
        // 1. Check permission
        if !self.has_permission(permissions, "conversations:update_user_assignee") {
            return Err(ApiError::Forbidden(
                "Missing permission: conversations:update_user_assignee".to_string(),
            ));
        }

        // 2. Verify conversation exists
        let conversation = self
            .db
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Conversation {} not found", conversation_id))
            })?;

        // 3. Verify target agent exists
        let target_agent = self
            .db
            .get_agent_by_user_id(target_agent_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Agent {} not found", target_agent_id))
            })?;

        // 4. Assign to database
        self.db
            .assign_conversation_to_user(conversation_id, target_agent_id, assigning_agent_id)
            .await?;

        // 5. Add target agent as participant (ignore if already exists)
        let _ = self
            .db
            .add_conversation_participant(conversation_id, target_agent_id, Some(assigning_agent_id))
            .await;

        // 6. Record in history
        let history = AssignmentHistory::new(
            conversation_id.to_string(),
            Some(target_agent_id.to_string()),
            None,
            assigning_agent_id.to_string(),
        );
        self.db.record_assignment(&history).await?;

        // 7. Publish event
        self.event_bus.publish(SystemEvent::ConversationAssigned {
            conversation_id: conversation_id.to_string(),
            assigned_user_id: Some(target_agent_id.to_string()),
            assigned_team_id: None,
            assigned_by: assigning_agent_id.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        // 8. Send notification to target agent (async, fire-and-forget)
        let notification_service = self.notification_service.clone();
        let user_id = target_agent_id.to_string();
        let conv_id = conversation_id.to_string();
        tokio::spawn(async move {
            let _ = notification_service.notify_assignment(&user_id, &conv_id).await;
        });

        // 9. Return updated conversation
        self.db
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::Internal("Conversation disappeared after assignment".to_string())
            })
    }

    // User Story 3: Team assignment
    pub async fn assign_conversation_to_team(
        &self,
        conversation_id: &str,
        team_id: &str,
        assigning_agent_id: &str,
        permissions: &[Permission],
    ) -> ApiResult<Conversation> {
        // 1. Check permission
        if !self.has_permission(permissions, "conversations:update_team_assignee") {
            return Err(ApiError::Forbidden(
                "Missing permission: conversations:update_team_assignee".to_string(),
            ));
        }

        // 2. Verify conversation exists
        let conversation = self
            .db
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Conversation {} not found", conversation_id))
            })?;

        // 3. Verify team exists
        let team = self.db.get_team_by_id(team_id).await?.ok_or_else(|| {
            ApiError::NotFound(format!("Team {} not found", team_id))
        })?;

        // 4. Assign to database
        self.db
            .assign_conversation_to_team(conversation_id, team_id, assigning_agent_id)
            .await?;

        // 5. Apply team SLA (stub)
        self.apply_team_sla(conversation_id, team_id).await?;

        // 6. Record in history
        let history = AssignmentHistory::new(
            conversation_id.to_string(),
            None,
            Some(team_id.to_string()),
            assigning_agent_id.to_string(),
        );
        self.db.record_assignment(&history).await?;

        // 7. Publish event
        self.event_bus.publish(SystemEvent::ConversationAssigned {
            conversation_id: conversation_id.to_string(),
            assigned_user_id: None,
            assigned_team_id: Some(team_id.to_string()),
            assigned_by: assigning_agent_id.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        // 8. Return updated conversation
        self.db
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::Internal("Conversation disappeared after assignment".to_string())
            })
    }

    // Stub method for SLA application (Feature 008 will implement)
    async fn apply_team_sla(&self, conversation_id: &str, team_id: &str) -> ApiResult<()> {
        tracing::info!(
            "STUB: Would apply SLA policy for team {} to conversation {}",
            team_id,
            conversation_id
        );
        // TODO: Feature 008 will implement actual SLA application
        Ok(())
    }

    // User Story 4: Manual unassignment
    pub async fn unassign_conversation(
        &self,
        conversation_id: &str,
        agent_id: &str,
    ) -> ApiResult<Conversation> {
        // 1. Get conversation
        let conversation = self
            .db
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Conversation {} not found", conversation_id))
            })?;

        // 2. Verify agent is currently assigned
        if conversation.assigned_user_id.as_ref() != Some(&agent_id.to_string()) {
            return Err(ApiError::BadRequest(
                "You are not assigned to this conversation".to_string(),
            ));
        }

        // 3. Unassign from database
        self.db.unassign_conversation_user(conversation_id).await?;

        // 4. Publish event
        self.event_bus.publish(SystemEvent::ConversationUnassigned {
            conversation_id: conversation_id.to_string(),
            previous_assigned_user_id: Some(agent_id.to_string()),
            previous_assigned_team_id: conversation.assigned_team_id.clone(),
            unassigned_by: agent_id.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        // 5. Return updated conversation
        self.db
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::Internal("Conversation disappeared after unassignment".to_string())
            })
    }

    // User Story 5: Automatic unassignment on away
    pub async fn auto_unassign_on_away(&self, agent_id: &str) -> ApiResult<Vec<Conversation>> {
        // 1. Get all open/snoozed conversations assigned to agent before unassignment
        let (conversations, _) = self
            .db
            .get_user_assigned_conversations(agent_id, 1000, 0)
            .await?;

        // Filter to only open/snoozed
        let open_conversations: Vec<_> = conversations
            .into_iter()
            .filter(|c| {
                matches!(
                    c.status,
                    ConversationStatus::Open | ConversationStatus::Snoozed
                )
            })
            .collect();

        // 2. Batch unassign
        let count = self
            .db
            .unassign_agent_open_conversations(agent_id)
            .await?;

        // 3. Publish ConversationUnassigned event for each
        for conversation in &open_conversations {
            self.event_bus.publish(SystemEvent::ConversationUnassigned {
                conversation_id: conversation.id.clone(),
                previous_assigned_user_id: Some(agent_id.to_string()),
                previous_assigned_team_id: conversation.assigned_team_id.clone(),
                unassigned_by: agent_id.to_string(), // System-triggered but by agent's action
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }

        tracing::info!(
            "Auto-unassigned {} conversations for agent {}",
            count,
            agent_id
        );

        // 4. Return list of unassigned conversations
        Ok(open_conversations)
    }

    // Get user permissions from database
    pub async fn get_user_permissions(&self, user_id: &str) -> ApiResult<Vec<Permission>> {
        self.db.get_user_permissions(user_id).await
    }

    // Check if user is a team member
    pub async fn is_team_member(&self, team_id: &str, user_id: &str) -> ApiResult<bool> {
        self.db.is_team_member(team_id, user_id).await
    }

    // Get unassigned conversations
    pub async fn get_unassigned_conversations(
        &self,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Conversation>, i64)> {
        self.db.get_unassigned_conversations(limit, offset).await
    }

    // Get conversations assigned to a user
    pub async fn get_user_assigned_conversations(
        &self,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Conversation>, i64)> {
        self.db.get_user_assigned_conversations(user_id, limit, offset).await
    }

    // Get team conversations
    pub async fn get_team_conversations(
        &self,
        team_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Conversation>, i64)> {
        self.db.get_team_conversations(team_id, limit, offset).await
    }

    // Update agent availability
    pub async fn update_agent_availability(
        &self,
        user_id: &str,
        status: AgentAvailability,
    ) -> ApiResult<()> {
        self.db.update_agent_availability(user_id, status).await
    }
}
