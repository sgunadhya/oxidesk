# Removing DB from AppState - Completion Plan

## Current Status
All major services refactored to use repositories. Need to complete final steps to remove `db` field from AppState.

## Remaining Work

### 1. Create NotificationRepository (DONE - Need to implement)
- [x] Create trait at `src/domain/ports/notification_repository.rs`
- [ ] Implement for Database in `src/database/notification.rs`
- [ ] Update NotificationService to use repository

### 2. Fix API Method Calls
Replace these `state.db` calls with service calls:

**src/api/users.rs:245** - Direct pool access
```rust
.execute(state.db.pool())  // Line 245
```
→ Need to check context and route through appropriate service

**src/api/users.rs** - get_user_roles (3 times at lines 74, 155, 228)
```rust
state.db.get_user_roles(&user.id)
```
→ Replace with `state.role_service.get_user_roles(&user.id)`

**src/api/users.rs:233** - count_admin_users
```rust
state.db.count_admin_users()
```
→ Replace with `state.user_service.count_admin_users()`

**src/api/conversations.rs** - get_user_teams (3 times at lines 75, 135, 251)
```rust
state.db.get_user_teams(&auth_user.user.id)
```
→ Replace with `state.team_service.get_user_teams(&auth_user.user.id)`

**src/api/api_keys.rs:155** - revoke_api_key
```rust
state.db.revoke_api_key(&agent_id)
```
→ Replace with `state.agent_service.revoke_api_key(&agent_id)`

**src/api/api_keys.rs:219** - count_api_keys
```rust
state.db.count_api_keys()
```
→ Replace with `state.agent_service.count_api_keys()`

**src/api/notifications.rs:134** - get_unread_count
```rust
state.db.get_unread_count(&user.user.id)
```
→ Replace with notification service method

**src/api/notifications.rs:191** - mark_notification_as_read
```rust
state.db.mark_notification_as_read(&id)
```
→ Replace with notification service method

### 3. Update AgentService Initialization
All places where `AgentService::new` is called need to add `api_key_repo` parameter:
- src/main.rs
- All test files that create AgentService

### 4. Remove `db` field from AppState
Edit `src/api/middleware/auth.rs`:
```rust
pub struct AppState {
    // pub db: Database,  ← REMOVE THIS
    pub session_duration_hours: i64,
    ...
}
```

### 5. Update main.rs AppState initialization
Remove `db: db.clone(),` from AppState creation

## Commands to Execute

```bash
# 1. Replace API calls
sed -i '' 's/state\.db\.get_user_roles/state.role_service.get_user_roles/g' src/api/*.rs
sed -i '' 's/state\.db\.count_admin_users/state.user_service.count_admin_users/g' src/api/*.rs
sed -i '' 's/state\.db\.get_user_teams/state.team_service.get_user_teams/g' src/api/*.rs
sed -i '' 's/state\.db\.revoke_api_key/state.agent_service.revoke_api_key/g' src/api/*.rs
sed -i '' 's/state\.db\.count_api_keys/state.agent_service.count_api_keys/g' src/api/*.rs

# 2. Build and test
cargo build
cargo test
```

## Critical Files to Update
1. src/domain/ports/notification_repository.rs (create)
2. src/database/notification.rs (implement trait)
3. src/services/notification_service.rs (add repository, add methods)
4. src/api/middleware/auth.rs (remove db field from AppState)
5. src/main.rs (update AppState init, update AgentService init)
6. src/api/users.rs (fix pool() access at line 245)
7. All test files with AgentService::new
