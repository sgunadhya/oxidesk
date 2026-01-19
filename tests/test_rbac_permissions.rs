/// Unit tests for PermissionService
/// Tests permission checking logic and aggregation across multiple roles
use oxidesk::domain::entities::Role;
use oxidesk::application::services::PermissionService;

#[test]
fn test_has_permission_single_role() {
    let role = Role {
        id: "role-1".to_string(),
        name: "Support Agent".to_string(),
        description: Some("Support role".to_string()),
        permissions: vec![
            "conversations:read_assigned".to_string(),
            "messages:write".to_string(),
        ],
        is_protected: false,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    };

    let roles = vec![role];

    // Should have granted permissions
    assert!(PermissionService::has_permission(
        &roles,
        "conversations:read_assigned"
    ));
    assert!(PermissionService::has_permission(&roles, "messages:write"));

    // Should not have other permissions
    assert!(!PermissionService::has_permission(
        &roles,
        "conversations:read_all"
    ));
    assert!(!PermissionService::has_permission(&roles, "roles:manage"));
}

#[test]
fn test_has_permission_multiple_roles_union() {
    let support_role = Role {
        id: "role-1".to_string(),
        name: "Support Agent".to_string(),
        description: Some("Support role".to_string()),
        permissions: vec![
            "conversations:read_assigned".to_string(),
            "messages:write".to_string(),
        ],
        is_protected: false,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    };

    let manager_role = Role {
        id: "role-2".to_string(),
        name: "Manager".to_string(),
        description: Some("Manager role".to_string()),
        permissions: vec![
            "conversations:read_all".to_string(),
            "conversations:update_team_assignee".to_string(),
        ],
        is_protected: false,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    };

    let roles = vec![support_role, manager_role];

    // Should have permissions from BOTH roles (union)
    assert!(PermissionService::has_permission(
        &roles,
        "conversations:read_assigned"
    ));
    assert!(PermissionService::has_permission(&roles, "messages:write"));
    assert!(PermissionService::has_permission(
        &roles,
        "conversations:read_all"
    ));
    assert!(PermissionService::has_permission(
        &roles,
        "conversations:update_team_assignee"
    ));

    // Should not have permissions from neither role
    assert!(!PermissionService::has_permission(&roles, "roles:manage"));
}

#[test]
fn test_has_any_permission() {
    let role = Role {
        id: "role-1".to_string(),
        name: "Support Agent".to_string(),
        description: Some("Support role".to_string()),
        permissions: vec!["conversations:read_assigned".to_string()],
        is_protected: false,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    };

    let roles = vec![role];

    // Has at least one of the permissions
    assert!(PermissionService::has_any_permission(
        &roles,
        &["conversations:read_all", "conversations:read_assigned"]
    ));

    // Does not have any of the permissions
    assert!(!PermissionService::has_any_permission(
        &roles,
        &["conversations:read_all", "roles:manage"]
    ));
}

#[test]
fn test_has_all_permissions() {
    let role = Role {
        id: "role-1".to_string(),
        name: "Support Agent".to_string(),
        description: Some("Support role".to_string()),
        permissions: vec![
            "conversations:read_assigned".to_string(),
            "messages:write".to_string(),
        ],
        is_protected: false,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    };

    let roles = vec![role];

    // Has all required permissions
    assert!(PermissionService::has_all_permissions(
        &roles,
        &["conversations:read_assigned", "messages:write"]
    ));

    // Missing one permission
    assert!(!PermissionService::has_all_permissions(
        &roles,
        &["conversations:read_assigned", "roles:manage"]
    ));
}

#[test]
fn test_empty_roles() {
    let roles = vec![];

    // No roles = no permissions
    assert!(!PermissionService::has_permission(
        &roles,
        "conversations:read_assigned"
    ));
    assert!(!PermissionService::has_any_permission(
        &roles,
        &["conversations:read_all"]
    ));
    assert!(!PermissionService::has_all_permissions(
        &roles,
        &["messages:write"]
    ));
}

#[test]
fn test_role_with_empty_permissions() {
    let role = Role {
        id: "role-1".to_string(),
        name: "Empty Role".to_string(),
        description: Some("Role with no permissions".to_string()),
        permissions: vec![],
        is_protected: false,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    };

    let roles = vec![role];

    // No permissions in role
    assert!(!PermissionService::has_permission(
        &roles,
        "conversations:read_assigned"
    ));
}
