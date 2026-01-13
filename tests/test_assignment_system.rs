// Integration tests for conversation assignment system (Feature 004)
use oxidesk::models::{
    conversation::ConversationStatus, team::TeamMemberRole, user::AgentAvailability, Team,
};

mod helpers;
use helpers::*;

#[tokio::test]
async fn test_create_team_and_add_members() {
    let db = setup_test_db().await;

    // Create a team
    let team = Team::new(
        "Support Team".to_string(),
        Some("Customer support team".to_string()),
    );
    db.create_team(&team).await.expect("Failed to create team");

    // Verify team was created
    let retrieved_team = db
        .get_team_by_id(&team.id)
        .await
        .expect("Failed to get team")
        .expect("Team not found");
    assert_eq!(retrieved_team.name, "Support Team");

    // Create test agent
    let agent = create_test_agent(&db, "agent@example.com", "Agent One").await;

    // Add agent to team
    db.add_team_member(&team.id, &agent.user_id, TeamMemberRole::Member)
        .await
        .expect("Failed to add team member");

    // Verify membership
    let is_member = db
        .is_team_member(&team.id, &agent.user_id)
        .await
        .expect("Failed to check membership");
    assert!(is_member);

    // Get team members
    let members = db
        .get_team_members(&team.id)
        .await
        .expect("Failed to get team members");
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].id, agent.user_id);
}

#[tokio::test]
async fn test_conversation_assignment_flow() {
    let db = setup_test_db().await;

    // Setup: Create agent and conversation
    let agent = create_test_agent(&db, "agent@example.com", "Agent One").await;
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Step 1: Assign conversation to agent
    db.assign_conversation_to_user(&conversation.id, &agent.user_id, &agent.user_id)
        .await
        .expect("Failed to assign conversation");

    // Verify assignment
    let updated_conv = db
        .get_conversation_by_id(&conversation.id)
        .await
        .expect("Failed to get conversation")
        .expect("Conversation not found");
    assert_eq!(updated_conv.assigned_user_id, Some(agent.user_id.clone()));
    assert!(updated_conv.assigned_at.is_some());
    assert_eq!(updated_conv.assigned_by, Some(agent.user_id.clone()));

    // Step 2: Verify conversation appears in agent's assigned list
    let (assigned_convs, count) = db
        .get_user_assigned_conversations(&agent.user_id, 10, 0)
        .await
        .expect("Failed to get assigned conversations");
    assert_eq!(count, 1);
    assert_eq!(assigned_convs.len(), 1);
    assert_eq!(assigned_convs[0].id, conversation.id);

    // Step 3: Unassign conversation
    db.unassign_conversation_user(&conversation.id)
        .await
        .expect("Failed to unassign conversation");

    // Verify unassignment
    let unassigned_conv = db
        .get_conversation_by_id(&conversation.id)
        .await
        .expect("Failed to get conversation")
        .expect("Conversation not found");
    assert_eq!(unassigned_conv.assigned_user_id, None);

    // Step 4: Verify conversation appears in unassigned list
    let (unassigned_convs, count) = db
        .get_unassigned_conversations(10, 0)
        .await
        .expect("Failed to get unassigned conversations");
    assert_eq!(count, 1);
    assert_eq!(unassigned_convs.len(), 1);
    assert_eq!(unassigned_convs[0].id, conversation.id);
}

#[tokio::test]
async fn test_team_assignment_and_inbox() {
    let db = setup_test_db().await;

    // Setup
    let agent = create_test_agent(&db, "agent@example.com", "Agent One").await;
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    let team = Team::new("Support Team".to_string(), None);
    db.create_team(&team).await.expect("Failed to create team");
    db.add_team_member(&team.id, &agent.user_id, TeamMemberRole::Member)
        .await
        .expect("Failed to add team member");

    // Assign conversation to team
    db.assign_conversation_to_team(&conversation.id, &team.id, &agent.user_id)
        .await
        .expect("Failed to assign to team");

    // Verify assignment
    let updated_conv = db
        .get_conversation_by_id(&conversation.id)
        .await
        .expect("Failed to get conversation")
        .expect("Conversation not found");
    assert_eq!(updated_conv.assigned_team_id, Some(team.id.clone()));

    // Verify conversation appears in team inbox
    let (team_convs, count) = db
        .get_team_conversations(&team.id, 10, 0)
        .await
        .expect("Failed to get team conversations");
    assert_eq!(count, 1);
    assert_eq!(team_convs.len(), 1);
    assert_eq!(team_convs[0].id, conversation.id);
}

#[tokio::test]
async fn test_auto_unassignment_on_away() {
    let db = setup_test_db().await;

    // Setup: Create agent and two conversations (one open, one resolved)
    let agent = create_test_agent(&db, "agent@example.com", "Agent One").await;
    let contact = create_test_contact(&db, "customer@example.com").await;

    let open_conv = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    let resolved_conv = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Resolved,
    )
    .await;

    // Assign both conversations to agent
    db.assign_conversation_to_user(&open_conv.id, &agent.user_id, &agent.user_id)
        .await
        .expect("Failed to assign open conversation");
    db.assign_conversation_to_user(&resolved_conv.id, &agent.user_id, &agent.user_id)
        .await
        .expect("Failed to assign resolved conversation");

    // Change agent availability to away_and_reassigning
    db.update_agent_availability(&agent.user_id, AgentAvailability::AwayAndReassigning)
        .await
        .expect("Failed to update availability");

    // Trigger auto-unassignment
    let count = db
        .unassign_agent_open_conversations(&agent.user_id)
        .await
        .expect("Failed to auto-unassign");
    assert_eq!(count, 1); // Only open conversation should be unassigned

    // Verify open conversation was unassigned
    let open_conv_updated = db
        .get_conversation_by_id(&open_conv.id)
        .await
        .expect("Failed to get conversation")
        .expect("Conversation not found");
    assert_eq!(open_conv_updated.assigned_user_id, None);

    // Verify resolved conversation remains assigned
    let resolved_conv_updated = db
        .get_conversation_by_id(&resolved_conv.id)
        .await
        .expect("Failed to get conversation")
        .expect("Conversation not found");
    assert_eq!(
        resolved_conv_updated.assigned_user_id,
        Some(agent.user_id.clone())
    );
}

#[tokio::test]
async fn test_assignment_history_audit_trail() {
    let db = setup_test_db().await;

    // Setup
    let agent1 = create_test_agent(&db, "agent1@example.com", "Agent One").await;
    let agent2 = create_test_agent(&db, "agent2@example.com", "Agent Two").await;
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Assignment 1: Agent 1 self-assigns
    db.assign_conversation_to_user(&conversation.id, &agent1.user_id, &agent1.user_id)
        .await
        .expect("Failed to assign conversation");

    let history1 = oxidesk::models::AssignmentHistory::new(
        conversation.id.clone(),
        Some(agent1.user_id.clone()),
        None,
        agent1.user_id.clone(),
    );
    db.record_assignment(&history1)
        .await
        .expect("Failed to record history");

    // Assignment 2: Agent 1 assigns to Agent 2
    db.assign_conversation_to_user(&conversation.id, &agent2.user_id, &agent1.user_id)
        .await
        .expect("Failed to assign conversation");

    let history2 = oxidesk::models::AssignmentHistory::new(
        conversation.id.clone(),
        Some(agent2.user_id.clone()),
        None,
        agent1.user_id.clone(),
    );
    db.record_assignment(&history2)
        .await
        .expect("Failed to record history");

    // Get assignment history
    let history = db
        .get_assignment_history(&conversation.id)
        .await
        .expect("Failed to get assignment history");

    // Verify history records
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].id, history2.id); // Most recent first
    assert_eq!(history[0].assigned_user_id, Some(agent2.user_id.clone()));
    assert_eq!(history[0].assigned_by, agent1.user_id);

    assert_eq!(history[1].id, history1.id);
    assert_eq!(history[1].assigned_user_id, Some(agent1.user_id.clone()));
    assert_eq!(history[1].assigned_by, agent1.user_id);
}

#[tokio::test]
async fn test_conversation_participants() {
    let db = setup_test_db().await;

    // Setup
    let agent = create_test_agent(&db, "agent@example.com", "Agent One").await;
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Add agent as participant
    db.add_conversation_participant(&conversation.id, &agent.user_id, Some(&agent.user_id))
        .await
        .expect("Failed to add participant");

    // Get participants
    let participants = db
        .get_conversation_participants(&conversation.id)
        .await
        .expect("Failed to get participants");

    assert_eq!(participants.len(), 1);
    assert_eq!(participants[0].id, agent.user_id);

    // Try to add duplicate participant (should fail)
    let result = db
        .add_conversation_participant(&conversation.id, &agent.user_id, Some(&agent.user_id))
        .await;
    assert!(result.is_err()); // Should fail due to unique constraint
}

#[tokio::test]
async fn test_agent_availability_states() {
    let db = setup_test_db().await;

    let agent = create_test_agent(&db, "agent@example.com", "Agent One").await;

    // Test all availability states
    for status in [
        AgentAvailability::Online,
        AgentAvailability::Away,
        AgentAvailability::AwayAndReassigning,
    ] {
        db.update_agent_availability(&agent.user_id, status)
            .await
            .expect("Failed to update availability");

        let retrieved_status = db
            .get_agent_availability(&agent.user_id)
            .await
            .expect("Failed to get availability");

        assert_eq!(retrieved_status, status);
    }
}
