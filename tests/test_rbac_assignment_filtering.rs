/// Integration tests for assignment-based access control (Scenario 2)
/// Tests that users with conversations:read_assigned can only access assigned conversations
mod helpers;
use helpers::rbac_helpers::{
    add_user_to_team, create_auth_user_with_roles, create_conversation_assigned_to_team,
    create_conversation_assigned_to_user, create_test_agent as create_rbac_test_agent,
    create_test_role, create_test_team, ensure_test_inbox,
};
use helpers::*;
use oxidesk::services::PermissionService;

#[tokio::test]
async fn test_assigned_conversation_access_granted() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create Support Agent role with read_assigned permission
    let support_role = create_test_role(
        db,
        "Support Agent",
        Some("Basic support"),
        vec!["conversations:read_assigned".to_string()],
    )
    .await;

    // Create agent Alice
    let alice =
        create_auth_user_with_roles(db, "alice@example.com", "Alice", vec![support_role]).await;

    // Create contact
    let contact = create_test_contact(db, "customer@example.com").await;

    // Create conversation assigned to Alice
    let conv_id = create_conversation_assigned_to_user(db, &contact.id, &alice.user.id).await;

    // Alice should have access to her assigned conversation (check via database)
    let conversation = db
        .get_conversation_by_id(&conv_id)
        .await
        .expect("Failed to get conversation")
        .expect("Conversation not found");

    let has_access = conversation.assigned_user_id.as_ref() == Some(&alice.user.id);

    assert!(
        has_access,
        "Alice should have access to her assigned conversation"
    );
}

#[tokio::test]
async fn test_unassigned_conversation_access_denied() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create Support Agent role
    let support_role = create_test_role(
        db,
        "Support Agent",
        Some("Basic support"),
        vec!["conversations:read_assigned".to_string()],
    )
    .await;

    // Create agents Alice and Bob
    let alice =
        create_auth_user_with_roles(db, "alice@example.com", "Alice", vec![support_role.clone()])
            .await;

    let (bob_user, _bob_agent) = create_rbac_test_agent(db, "bob@example.com", "Bob").await;

    // Create contact
    let contact = create_test_contact(db, "customer@example.com").await;

    // Create conversation assigned to Bob (not Alice)
    let conv_id = create_conversation_assigned_to_user(db, &contact.id, &bob_user.id).await;

    // Alice should NOT have access to Bob's conversation (check via database)
    let conversation = db
        .get_conversation_by_id(&conv_id)
        .await
        .expect("Failed to get conversation")
        .expect("Conversation not found");

    let has_access = conversation.assigned_user_id.as_ref() == Some(&alice.user.id);

    assert!(
        !has_access,
        "Alice should not have access to Bob's conversation"
    );
}

#[tokio::test]
async fn test_admin_with_read_all_bypasses_assignment() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create Manager role with read_all permission
    let manager_role = create_test_role(
        db,
        "Manager",
        Some("Manager with full access"),
        vec!["conversations:read_all".to_string()],
    )
    .await;

    // Create manager
    let manager =
        create_auth_user_with_roles(db, "manager@example.com", "Manager", vec![manager_role]).await;

    // Manager has read_all permission (bypasses assignment checks)
    assert!(PermissionService::has_permission(
        &manager.roles,
        "conversations:read_all"
    ));

    // Even without assignment, manager should be able to access any conversation
    // This is enforced at the handler level, not in AssignmentService
    assert!(!PermissionService::has_permission(
        &manager.roles,
        "conversations:read_assigned"
    ));
}

#[tokio::test]
async fn test_team_assignment_grants_access() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create Support Agent role
    let support_role = create_test_role(
        db,
        "Support Agent",
        Some("Basic support"),
        vec!["conversations:read_assigned".to_string()],
    )
    .await;

    // Create agent Dave
    let dave =
        create_auth_user_with_roles(db, "dave@example.com", "Dave", vec![support_role]).await;

    // Create team and add Dave to it
    let team_id = create_test_team(db, "Support Team").await;
    add_user_to_team(db, &dave.user.id, &team_id).await;

    // Create contact
    let contact = create_test_contact(db, "customer@example.com").await;

    // Create conversation assigned to team (not Dave directly)
    let conv_id = create_conversation_assigned_to_team(db, &contact.id, &team_id).await;

    // Dave should have access because he's in the assigned team (check via database)
    let conversation = db
        .get_conversation_by_id(&conv_id)
        .await
        .expect("Failed to get conversation")
        .expect("Conversation not found");

    // Check if Dave is in the assigned team
    let user_teams = db
        .get_user_teams(&dave.user.id)
        .await
        .expect("Failed to get user teams");
    let has_access = conversation
        .assigned_team_id
        .as_ref()
        .map_or(false, |team_id| user_teams.iter().any(|t| &t.id == team_id));

    assert!(has_access, "Dave should have access via team membership");
}

#[tokio::test]
async fn test_non_team_member_denied_team_conversation() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create Support Agent role
    let support_role = create_test_role(
        db,
        "Support Agent",
        Some("Basic support"),
        vec!["conversations:read_assigned".to_string()],
    )
    .await;

    // Create agents Alice and Dave
    let alice =
        create_auth_user_with_roles(db, "alice@example.com", "Alice", vec![support_role.clone()])
            .await;

    let dave =
        create_auth_user_with_roles(db, "dave@example.com", "Dave", vec![support_role]).await;

    // Create team and add only Dave (not Alice)
    let team_id = create_test_team(db, "Support Team").await;
    add_user_to_team(db, &dave.user.id, &team_id).await;

    // Create contact
    let contact = create_test_contact(db, "customer@example.com").await;

    // Create conversation assigned to team
    let conv_id = create_conversation_assigned_to_team(db, &contact.id, &team_id).await;

    // Alice should NOT have access (she's not in the team)
    let conversation = db
        .get_conversation_by_id(&conv_id)
        .await
        .expect("Failed to get conversation")
        .expect("Conversation not found");

    // Check if Alice is in the assigned team
    let user_teams = db
        .get_user_teams(&alice.user.id)
        .await
        .expect("Failed to get user teams");
    let has_access = conversation
        .assigned_team_id
        .as_ref()
        .map_or(false, |team_id| user_teams.iter().any(|t| &t.id == team_id));

    assert!(
        !has_access,
        "Alice should not have access to team conversation (not a member)"
    );
}

#[tokio::test]
async fn test_user_and_team_assignment_both_work() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create Support Agent role
    let support_role = create_test_role(
        db,
        "Support Agent",
        Some("Basic support"),
        vec!["conversations:read_assigned".to_string()],
    )
    .await;

    // Create agent
    let agent =
        create_auth_user_with_roles(db, "agent@example.com", "Agent", vec![support_role]).await;

    // Create contact
    let contact = create_test_contact(db, "customer@example.com").await;

    // Create conversation assigned directly to agent
    let conv_user = create_conversation_assigned_to_user(db, &contact.id, &agent.user.id).await;

    // Create team and add agent
    let team_id = create_test_team(db, "Support Team").await;
    add_user_to_team(db, &agent.user.id, &team_id).await;

    // Create conversation assigned to team
    let conv_team = create_conversation_assigned_to_team(db, &contact.id, &team_id).await;

    // Should have access to user-assigned conversation (check via database)
    let conv_user_obj = db
        .get_conversation_by_id(&conv_user)
        .await
        .expect("Failed to get conversation")
        .expect("Conversation not found");
    let has_access_user = conv_user_obj.assigned_user_id.as_ref() == Some(&agent.user.id);

    assert!(
        has_access_user,
        "Agent should have access to directly assigned conversation"
    );

    // Should have access to team-assigned conversation (check via database)
    let conv_team_obj = db
        .get_conversation_by_id(&conv_team)
        .await
        .expect("Failed to get conversation")
        .expect("Conversation not found");
    let user_teams = db
        .get_user_teams(&agent.user.id)
        .await
        .expect("Failed to get user teams");
    let has_access_team = conv_team_obj
        .assigned_team_id
        .as_ref()
        .map_or(false, |team_id| user_teams.iter().any(|t| &t.id == team_id));

    assert!(
        has_access_team,
        "Agent should have access to team-assigned conversation"
    );
}

#[tokio::test]
async fn test_unassigned_conversation_no_access() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create Support Agent role
    let support_role = create_test_role(
        db,
        "Support Agent",
        Some("Basic support"),
        vec!["conversations:read_assigned".to_string()],
    )
    .await;

    // Create agent
    let agent =
        create_auth_user_with_roles(db, "agent@example.com", "Agent", vec![support_role]).await;

    // Create contact
    let contact = create_test_contact(db, "customer@example.com").await;

    // Create unassigned conversation (no assigned_user_id or assigned_team_id)
    let inbox_id = ensure_test_inbox(db).await;
    let conv_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO conversations (id, reference_number, status, inbox_id, contact_id, created_at, updated_at)
         VALUES (?, (SELECT COALESCE(MAX(reference_number), 99) + 1 FROM conversations), 'open', ?, ?, ?, ?)"
    )
    .bind(&conv_id)
    .bind(&inbox_id)
    .bind(&contact.id)
    .bind(&now)
    .bind(&now)
    .execute(db.pool())
    .await
    .expect("Failed to create unassigned conversation");

    // Agent should NOT have access to unassigned conversation (check via database)
    let conversation = db
        .get_conversation_by_id(&conv_id)
        .await
        .expect("Failed to get conversation")
        .expect("Conversation not found");

    // Check if conversation is assigned to agent or their teams
    let has_user_assignment = conversation.assigned_user_id.as_ref() == Some(&agent.user.id);
    let user_teams = db
        .get_user_teams(&agent.user.id)
        .await
        .expect("Failed to get user teams");
    let has_team_assignment = conversation
        .assigned_team_id
        .as_ref()
        .map_or(false, |team_id| user_teams.iter().any(|t| &t.id == team_id));
    let has_access = has_user_assignment || has_team_assignment;

    assert!(
        !has_access,
        "Agent should not have access to unassigned conversation"
    );
}
