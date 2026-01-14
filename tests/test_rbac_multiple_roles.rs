/// Integration tests for multiple roles permission aggregation (Scenario 4)
/// Tests that users with multiple roles have permissions from ALL roles (union)
mod helpers;
use helpers::rbac_helpers::{create_auth_user_with_roles, create_test_role};
use helpers::*;
use oxidesk::services::PermissionService;

#[tokio::test]
async fn test_multiple_roles_permission_union() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create Support Agent role with limited permissions
    let support_role = create_test_role(
        db,
        "Support Agent",
        Some("Basic support team member"),
        vec![
            "conversations:read_assigned".to_string(),
            "messages:write".to_string(),
        ],
    )
    .await;

    // Create Manager role with elevated permissions
    let manager_role = create_test_role(
        db,
        "Manager",
        Some("Team manager"),
        vec![
            "conversations:read_all".to_string(),
            "conversations:update_team_assignee".to_string(),
        ],
    )
    .await;

    // Create user Carol with BOTH roles
    let carol = create_auth_user_with_roles(
        db,
        "carol@example.com",
        "Carol",
        vec![support_role.clone(), manager_role.clone()],
    )
    .await;

    // Carol should have permissions from Support Agent role
    assert!(PermissionService::has_permission(
        &carol.roles,
        "conversations:read_assigned"
    ));
    assert!(PermissionService::has_permission(
        &carol.roles,
        "messages:write"
    ));

    // Carol should have permissions from Manager role
    assert!(PermissionService::has_permission(
        &carol.roles,
        "conversations:read_all"
    ));
    assert!(PermissionService::has_permission(
        &carol.roles,
        "conversations:update_team_assignee"
    ));

    // Carol should NOT have permissions from neither role
    assert!(!PermissionService::has_permission(
        &carol.roles,
        "roles:manage"
    ));
}

#[tokio::test]
async fn test_user_with_single_role_limited_permissions() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create Support Agent role
    let support_role = create_test_role(
        db,
        "Support Agent",
        Some("Basic support team member"),
        vec![
            "conversations:read_assigned".to_string(),
            "messages:write".to_string(),
        ],
    )
    .await;

    // Create user Alice with only Support Agent role
    let alice =
        create_auth_user_with_roles(db, "alice@example.com", "Alice", vec![support_role]).await;

    // Alice should have permissions from Support Agent role
    assert!(PermissionService::has_permission(
        &alice.roles,
        "conversations:read_assigned"
    ));
    assert!(PermissionService::has_permission(
        &alice.roles,
        "messages:write"
    ));

    // Alice should NOT have Manager permissions
    assert!(!PermissionService::has_permission(
        &alice.roles,
        "conversations:read_all"
    ));
    assert!(!PermissionService::has_permission(
        &alice.roles,
        "conversations:update_team_assignee"
    ));
}

#[tokio::test]
async fn test_three_roles_aggregation() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create three different roles
    let role1 = create_test_role(
        db,
        "Role 1",
        None,
        vec!["permission:a".to_string(), "permission:b".to_string()],
    )
    .await;

    let role2 = create_test_role(
        db,
        "Role 2",
        None,
        vec!["permission:c".to_string(), "permission:d".to_string()],
    )
    .await;

    let role3 = create_test_role(db, "Role 3", None, vec!["permission:e".to_string()]).await;

    // Create user with all three roles
    let user = create_auth_user_with_roles(
        db,
        "multi@example.com",
        "Multi Role",
        vec![role1, role2, role3],
    )
    .await;

    // Should have permissions from all three roles
    assert!(PermissionService::has_permission(
        &user.roles,
        "permission:a"
    ));
    assert!(PermissionService::has_permission(
        &user.roles,
        "permission:b"
    ));
    assert!(PermissionService::has_permission(
        &user.roles,
        "permission:c"
    ));
    assert!(PermissionService::has_permission(
        &user.roles,
        "permission:d"
    ));
    assert!(PermissionService::has_permission(
        &user.roles,
        "permission:e"
    ));

    // Should not have permissions from no role
    assert!(!PermissionService::has_permission(
        &user.roles,
        "permission:f"
    ));
}

#[tokio::test]
async fn test_overlapping_permissions_in_multiple_roles() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create two roles with overlapping permissions
    let role1 = create_test_role(
        db,
        "Role 1",
        None,
        vec![
            "messages:write".to_string(),
            "conversations:read_assigned".to_string(),
        ],
    )
    .await;

    let role2 = create_test_role(
        db,
        "Role 2",
        None,
        vec![
            "messages:write".to_string(), // Duplicate permission
            "conversations:read_all".to_string(),
        ],
    )
    .await;

    // Create user with both roles
    let user = create_auth_user_with_roles(
        db,
        "overlap@example.com",
        "Overlap User",
        vec![role1, role2],
    )
    .await;

    // Should have all unique permissions (union handles duplicates)
    assert!(PermissionService::has_permission(
        &user.roles,
        "messages:write"
    ));
    assert!(PermissionService::has_permission(
        &user.roles,
        "conversations:read_assigned"
    ));
    assert!(PermissionService::has_permission(
        &user.roles,
        "conversations:read_all"
    ));
}

#[tokio::test]
async fn test_role_permissions_are_additive_across_roles() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create role 1 with initial permissions
    let role1 = create_test_role(
        db,
        "Role 1",
        None,
        vec!["permission:a".to_string(), "permission:b".to_string()],
    )
    .await;

    // Create role 2 with additional permissions
    let role2 = create_test_role(
        db,
        "Role 2",
        None,
        vec!["permission:c".to_string(), "permission:d".to_string()],
    )
    .await;

    // User with only role1
    let user1 =
        create_auth_user_with_roles(db, "user1@example.com", "User 1", vec![role1.clone()]).await;

    // User with both roles
    let user2 = create_auth_user_with_roles(
        db,
        "user2@example.com",
        "User 2",
        vec![role1.clone(), role2.clone()],
    )
    .await;

    // User 1 should only have role1 permissions
    assert!(PermissionService::has_permission(
        &user1.roles,
        "permission:a"
    ));
    assert!(PermissionService::has_permission(
        &user1.roles,
        "permission:b"
    ));
    assert!(!PermissionService::has_permission(
        &user1.roles,
        "permission:c"
    ));
    assert!(!PermissionService::has_permission(
        &user1.roles,
        "permission:d"
    ));

    // User 2 should have permissions from both roles
    assert!(PermissionService::has_permission(
        &user2.roles,
        "permission:a"
    ));
    assert!(PermissionService::has_permission(
        &user2.roles,
        "permission:b"
    ));
    assert!(PermissionService::has_permission(
        &user2.roles,
        "permission:c"
    ));
    assert!(PermissionService::has_permission(
        &user2.roles,
        "permission:d"
    ));
}
