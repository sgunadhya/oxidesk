use crate::domain::entities::Role;

/// Service for checking permissions based on user roles
/// Implements permission aggregation (union) from multiple roles
pub struct PermissionService;

impl PermissionService {
    /// Check if user has a specific permission
    /// Permission is granted if ANY assigned role has it (union logic)
    pub fn has_permission(roles: &[Role], permission: &str) -> bool {
        roles
            .iter()
            .any(|role| role.permissions.iter().any(|p| p == permission))
    }

    /// Check if user has any of the required permissions
    /// Returns true if user has at least one of the specified permissions
    pub fn has_any_permission(roles: &[Role], permissions: &[&str]) -> bool {
        permissions
            .iter()
            .any(|perm| Self::has_permission(roles, perm))
    }

    /// Check if user has all required permissions
    /// Returns true only if user has every specified permission
    pub fn has_all_permissions(roles: &[Role], permissions: &[&str]) -> bool {
        permissions
            .iter()
            .all(|perm| Self::has_permission(roles, perm))
    }

    /// Get all unique permissions from all roles (for debugging/admin purposes)
    pub fn get_all_permissions(roles: &[Role]) -> Vec<String> {
        let mut all_permissions = Vec::new();
        for role in roles {
            for permission in &role.permissions {
                if !all_permissions.contains(permission) {
                    all_permissions.push(permission.clone());
                }
            }
        }
        all_permissions.sort();
        all_permissions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::Role;

    fn create_test_role(name: &str, permissions: Vec<&str>) -> Role {
        Role {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: None,
            permissions: permissions.into_iter().map(|s| s.to_string()).collect(),
            is_protected: false,
            created_at: "2026-01-13T00:00:00Z".to_string(),
            updated_at: "2026-01-13T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_has_permission_single_role() {
        let roles = vec![create_test_role(
            "Support Agent",
            vec!["conversations:read_assigned", "messages:write"],
        )];

        assert!(PermissionService::has_permission(&roles, "messages:write"));
        assert!(PermissionService::has_permission(
            &roles,
            "conversations:read_assigned"
        ));
        assert!(!PermissionService::has_permission(
            &roles,
            "conversations:update_team_assignee"
        ));
    }

    #[test]
    fn test_has_permission_multiple_roles() {
        let roles = vec![
            create_test_role("Support Agent", vec!["messages:write"]),
            create_test_role(
                "Manager",
                vec![
                    "conversations:read_all",
                    "conversations:update_team_assignee",
                ],
            ),
        ];

        // Should have permissions from both roles (union)
        assert!(PermissionService::has_permission(&roles, "messages:write"));
        assert!(PermissionService::has_permission(
            &roles,
            "conversations:read_all"
        ));
        assert!(PermissionService::has_permission(
            &roles,
            "conversations:update_team_assignee"
        ));
        assert!(!PermissionService::has_permission(&roles, "sla:manage"));
    }

    #[test]
    fn test_has_any_permission() {
        let roles = vec![create_test_role(
            "Support Agent",
            vec!["conversations:read_assigned"],
        )];

        assert!(PermissionService::has_any_permission(
            &roles,
            &["conversations:read_all", "conversations:read_assigned",]
        ));

        assert!(!PermissionService::has_any_permission(
            &roles,
            &["conversations:read_all", "sla:manage",]
        ));
    }

    #[test]
    fn test_has_all_permissions() {
        let roles = vec![create_test_role(
            "Manager",
            vec![
                "conversations:read_all",
                "conversations:update_user_assignee",
                "conversations:update_team_assignee",
            ],
        )];

        assert!(PermissionService::has_all_permissions(
            &roles,
            &[
                "conversations:read_all",
                "conversations:update_user_assignee",
            ]
        ));

        assert!(!PermissionService::has_all_permissions(
            &roles,
            &["conversations:read_all", "sla:manage",]
        ));
    }

    #[test]
    fn test_get_all_permissions() {
        let roles = vec![
            create_test_role(
                "Support Agent",
                vec!["messages:write", "conversations:read_assigned"],
            ),
            create_test_role("Manager", vec!["conversations:read_all", "messages:write"]),
        ];

        let all_perms = PermissionService::get_all_permissions(&roles);
        assert_eq!(all_perms.len(), 3); // Unique permissions
        assert!(all_perms.contains(&"messages:write".to_string()));
        assert!(all_perms.contains(&"conversations:read_assigned".to_string()));
        assert!(all_perms.contains(&"conversations:read_all".to_string()));
    }

    #[test]
    fn test_empty_roles() {
        let roles: Vec<Role> = vec![];
        assert!(!PermissionService::has_permission(&roles, "any:permission"));
        assert!(!PermissionService::has_any_permission(
            &roles,
            &["any:permission"]
        ));
        assert!(!PermissionService::has_all_permissions(
            &roles,
            &["any:permission"]
        ));
    }
}
