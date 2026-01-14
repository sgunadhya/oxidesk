/// Integration tests for protected Admin role (Scenario 3)
/// Tests that Admin role cannot be modified or deleted
mod helpers;
use helpers::rbac_helpers::{create_auth_user_with_roles, create_test_role, ensure_admin_role};
use helpers::*;
use oxidesk::api::middleware::ApiError;
use oxidesk::models::{CreateRoleRequest, UpdateRoleRequest};
use oxidesk::services::role_service;
use sqlx::Row;

#[tokio::test]
async fn test_admin_role_is_protected() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Ensure Admin role exists
    let admin_role = ensure_admin_role(db).await;

    // Admin role should be marked as protected
    assert!(admin_role.is_protected, "Admin role should be protected");
    assert_eq!(admin_role.name, "Admin");
}

#[tokio::test]
async fn test_cannot_update_admin_role() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Get Admin role
    let admin_role = ensure_admin_role(db).await;

    // Create admin user
    let admin =
        create_auth_user_with_roles(db, "admin@example.com", "Admin", vec![admin_role.clone()])
            .await;

    // Attempt to update Admin role
    let update_request = UpdateRoleRequest {
        name: Some("Modified Admin".to_string()),
        description: Some("Modified description".to_string()),
        permissions: Some(vec!["conversations:read_assigned".to_string()]),
    };

    let result = role_service::update_role(db, &admin, &admin_role.id, update_request).await;

    // Should fail with Forbidden error
    assert!(result.is_err(), "Should not be able to update Admin role");
    match result.unwrap_err() {
        ApiError::Forbidden(msg) => {
            assert_eq!(msg, "Cannot modify Admin role");
        }
        other => panic!("Expected Forbidden error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_cannot_delete_admin_role() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Get Admin role
    let admin_role = ensure_admin_role(db).await;

    // Create admin user
    let admin =
        create_auth_user_with_roles(db, "admin@example.com", "Admin", vec![admin_role.clone()])
            .await;

    // Attempt to delete Admin role
    let result = role_service::delete(db, &admin, &admin_role.id).await;

    // Should fail with Forbidden error
    assert!(result.is_err(), "Should not be able to delete Admin role");
    match result.unwrap_err() {
        ApiError::Forbidden(msg) => {
            assert_eq!(msg, "Cannot modify Admin role");
        }
        other => panic!("Expected Forbidden error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_admin_role_has_is_protected_flag() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Ensure Admin role
    let admin_role = ensure_admin_role(db).await;

    // Verify is_protected flag directly
    assert_eq!(
        admin_role.is_protected, true,
        "Admin role should have is_protected=true"
    );

    // Verify via database query
    let rows =
        sqlx::query("SELECT CAST(is_protected AS INTEGER) as is_protected FROM roles WHERE id = ?")
            .bind(&admin_role.id)
            .fetch_all(db.pool())
            .await
            .expect("Failed to query role");

    assert_eq!(rows.len(), 1, "Admin role should exist in database");
    let is_protected: i32 = rows[0]
        .try_get("is_protected")
        .expect("Failed to get is_protected");
    assert_eq!(
        is_protected, 1,
        "is_protected should be 1 (true) in database"
    );
}

#[tokio::test]
async fn test_non_protected_role_can_be_updated() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create non-protected role
    let test_role = create_test_role(
        db,
        "Test Role",
        Some("Test description"),
        vec!["permission:test".to_string()],
    )
    .await;

    assert!(!test_role.is_protected, "Test role should not be protected");

    // Create admin user
    let admin_role = ensure_admin_role(db).await;
    let admin =
        create_auth_user_with_roles(db, "admin@example.com", "Admin", vec![admin_role]).await;

    // Update test role (should succeed)
    let update_request = UpdateRoleRequest {
        name: Some("Updated Test Role".to_string()),
        description: Some("Updated description".to_string()),
        permissions: Some(vec!["permission:updated".to_string()]),
    };

    let result = role_service::update_role(db, &admin, &test_role.id, update_request).await;

    assert!(
        result.is_ok(),
        "Should be able to update non-protected role"
    );
    let updated = result.unwrap();
    assert_eq!(updated.name, "Updated Test Role");
    assert_eq!(updated.permissions, vec!["permission:updated".to_string()]);
}

#[tokio::test]
async fn test_non_protected_role_can_be_deleted() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create non-protected role
    let test_role = create_test_role(
        db,
        "Test Role",
        Some("Test description"),
        vec!["permission:test".to_string()],
    )
    .await;

    // Create admin user
    let admin_role = ensure_admin_role(db).await;
    let admin =
        create_auth_user_with_roles(db, "admin@example.com", "Admin", vec![admin_role]).await;

    // Delete test role (should succeed)
    let result = role_service::delete(db, &admin, &test_role.id).await;

    assert!(
        result.is_ok(),
        "Should be able to delete non-protected role"
    );

    // Verify role is deleted
    let role_check = db.get_role_by_id(&test_role.id).await;
    assert!(
        role_check.is_ok() && role_check.unwrap().is_none(),
        "Role should be deleted from database"
    );
}

#[tokio::test]
async fn test_cannot_delete_role_with_assigned_users() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create non-protected role
    let test_role = create_test_role(
        db,
        "Test Role",
        Some("Test description"),
        vec!["permission:test".to_string()],
    )
    .await;

    // Create user with this role
    let _user =
        create_auth_user_with_roles(db, "user@example.com", "Test User", vec![test_role.clone()])
            .await;

    // Create admin user
    let admin_role = ensure_admin_role(db).await;
    let admin =
        create_auth_user_with_roles(db, "admin@example.com", "Admin", vec![admin_role]).await;

    // Attempt to delete role (should fail because user is assigned)
    let result = role_service::delete(db, &admin, &test_role.id).await;

    assert!(
        result.is_err(),
        "Should not be able to delete role with assigned users"
    );
    match result.unwrap_err() {
        ApiError::Conflict(msg) => {
            assert!(
                msg.contains("Cannot delete role"),
                "Error should mention role is in use"
            );
            assert!(
                msg.contains("agents currently assigned"),
                "Error should mention assigned users"
            );
        }
        other => panic!("Expected Conflict error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_admin_role_permissions_remain_intact() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Get Admin role
    let admin_role_before = ensure_admin_role(db).await;
    let permissions_before = admin_role_before.permissions.clone();

    // Create admin user
    let admin = create_auth_user_with_roles(
        db,
        "admin@example.com",
        "Admin",
        vec![admin_role_before.clone()],
    )
    .await;

    // Attempt to update (will fail)
    let update_request = UpdateRoleRequest {
        name: None,
        description: None,
        permissions: Some(vec!["limited:permission".to_string()]),
    };

    let _result =
        role_service::update_role(db, &admin, &admin_role_before.id, update_request).await;

    // Verify Admin role permissions are unchanged by re-fetching
    let admin_role_after = ensure_admin_role(db).await;
    assert_eq!(
        admin_role_after.permissions, permissions_before,
        "Admin role permissions should remain unchanged after failed update attempt"
    );
}

#[tokio::test]
async fn test_create_new_role_with_same_name_as_admin_fails() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Get Admin role
    let admin_role = ensure_admin_role(db).await;

    // Create admin user
    let admin =
        create_auth_user_with_roles(db, "admin@example.com", "Admin", vec![admin_role.clone()])
            .await;

    // Attempt to create new role with "Admin" name
    let create_request = CreateRoleRequest {
        name: "Admin".to_string(),
        description: Some("Duplicate admin".to_string()),
        permissions: vec!["test:permission".to_string()],
    };

    let result = role_service::create_role(db, &admin, create_request).await;

    // Should fail with Conflict error (duplicate name)
    assert!(
        result.is_err(),
        "Should not be able to create role with duplicate name"
    );
    match result.unwrap_err() {
        ApiError::Conflict(msg) => {
            assert!(
                msg.contains("already exists"),
                "Error should mention duplicate name"
            );
        }
        other => panic!("Expected Conflict error, got: {:?}", other),
    }
}
