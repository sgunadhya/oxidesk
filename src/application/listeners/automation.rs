use crate::application::services::{ConversationService, SlaService, TeamService};
use crate::AutomationService;
use crate::ConversationStatus;
use crate::EventBus;
use crate::SystemEvent;
use std::sync::Arc;
use tokio_stream::StreamExt;

pub async fn run_automation_listener(
    event_bus: Arc<dyn EventBus>,
    sla_service: SlaService,
    automation_service: Arc<AutomationService>,
    conversation_service: ConversationService,
    team_service: TeamService,
) {
    let automation_event_bus = event_bus;
    let automation_sla_service = sla_service;
    let automation_rule_service = automation_service;
    let automation_conversation_service = conversation_service;
    let automation_team_service = team_service;

    tracing::info!("Automation listener started (decoupled)");

    let mut receiver = automation_event_bus.subscribe();

    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(event) => {
                tracing::debug!("Automation listener received event: {:?}", event);

                // Process automation rules based on event
                match event {
                    SystemEvent::ConversationCreated {
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
                        if let Ok(conversation) = automation_conversation_service
                            .get_conversation(&conversation_id)
                            .await
                        {
                            if let Err(e) = automation_rule_service
                                .handle_conversation_event(
                                    "conversation.created",
                                    &conversation,
                                    "system",
                                )
                                .await
                            {
                                tracing::error!("Failed to execute automation rules for conversation created: {}", e);
                            }
                        }
                    }
                    SystemEvent::ConversationStatusChanged {
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
                        if new_status == ConversationStatus::Resolved {
                            if let Err(e) = automation_sla_service
                                .handle_conversation_resolved(&conversation_id, &timestamp)
                                .await
                            {
                                tracing::error!("Failed to handle resolution SLA: {}", e);
                            }
                        }

                        // Trigger automation rules for status change
                        if let Ok(conversation) = automation_conversation_service
                            .get_conversation(&conversation_id)
                            .await
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
                    SystemEvent::MessageReceived {
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
                        if let Ok(conversation) = automation_conversation_service
                            .get_conversation(&conversation_id)
                            .await
                        {
                            if let Err(e) = automation_rule_service
                                .handle_conversation_event(
                                    "conversation.message_received",
                                    &conversation,
                                    "system",
                                )
                                .await
                            {
                                tracing::error!(
                                    "Failed to execute automation rules for message received: {}",
                                    e
                                );
                            }
                        }
                    }
                    SystemEvent::MessageSent {
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
                        if let Ok(conversation) = automation_conversation_service
                            .get_conversation(&conversation_id)
                            .await
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
                    SystemEvent::MessageFailed {
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
                        if let Ok(conversation) = automation_conversation_service
                            .get_conversation(&conversation_id)
                            .await
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
                    SystemEvent::ConversationAssigned {
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
                                    if let Ok(team) =
                                        automation_team_service.get_team(team_id).await
                                    {
                                        if let Some(policy_id) = team.sla_policy_id {
                                            tracing::info!(
                                                    "Auto-applying SLA policy {} to conversation {} (assigned to team {})",
                                                    policy_id,
                                                    conversation_id,
                                                    team_id
                                                );

                                            match automation_sla_service
                                                .apply_sla(&conversation_id, &policy_id, &timestamp)
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
                        if let Ok(conversation) = automation_conversation_service
                            .get_conversation(&conversation_id)
                            .await
                        {
                            if let Err(e) = automation_rule_service
                                .handle_conversation_event(
                                    "conversation.assignment_changed",
                                    &conversation,
                                    &assigned_by,
                                )
                                .await
                            {
                                tracing::error!(
                                    "Failed to execute automation rules for assignment change: {}",
                                    e
                                );
                            }
                        }
                    }
                    SystemEvent::ConversationUnassigned {
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
                        if let Ok(conversation) = automation_conversation_service
                            .get_conversation(&conversation_id)
                            .await
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
                    }
                    SystemEvent::ConversationTagsChanged {
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
                        if let Ok(conversation) = automation_conversation_service
                            .get_conversation(&conversation_id)
                            .await
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
                    }
                    SystemEvent::AgentAvailabilityChanged {
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
                    }
                    SystemEvent::AgentLoggedIn {
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
                    SystemEvent::AgentLoggedOut {
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
                    SystemEvent::SlaBreached {
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
                        if let Ok(conversation) = automation_conversation_service
                            .get_conversation(&conversation_id)
                            .await
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
                    }
                    SystemEvent::ConversationPriorityChanged {
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
                        if let Ok(conversation) = automation_conversation_service
                            .get_conversation(&conversation_id)
                            .await
                        {
                            if let Err(e) = automation_rule_service
                                .handle_conversation_event(
                                    "conversation.priority_changed",
                                    &conversation,
                                    &updated_by,
                                )
                                .await
                            {
                                tracing::error!(
                                    "Failed to execute automation rules for priority change: {}",
                                    e
                                );
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Automation listener error: {}", e);
            }
        }
    }
}
