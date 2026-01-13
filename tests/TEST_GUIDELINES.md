# Test Guidelines

## Core Principles

### 1. NO DDL IN TESTS
**All database schema changes MUST be done via migrations only.**

❌ **NEVER DO THIS:**
```rust
// BAD - Don't create tables in tests
sqlx::query("CREATE TABLE users (id TEXT PRIMARY KEY, ...)")
    .execute(pool)
    .await?;

// BAD - Don't alter tables in tests
sqlx::query("ALTER TABLE users ADD COLUMN status TEXT")
    .execute(pool)
    .await?;
```

✅ **ALWAYS DO THIS:**
```rust
// GOOD - Let migrations handle schema
let test_db = setup_test_db().await; // Runs migrations automatically
let db = test_db.db();

// Then just insert test data
sqlx::query("INSERT INTO users (id, email, ...) VALUES (?, ?, ...)")
    .bind(user_id)
    .bind(email)
    .execute(db.pool())
    .await?;
```

### 2. Why No DDL in Tests?

1. **Single Source of Truth**: Migrations are the authoritative schema definition
2. **Migration Validation**: Tests verify that migrations work correctly
3. **Schema Drift Prevention**: Prevents test schema from diverging from production
4. **Refactoring Safety**: Schema changes propagate to tests automatically
5. **Real-world Testing**: Tests run against the same schema as production

### 3. Test Database Setup

**Current Implementation:**
```rust
// tests/helpers/test_db.rs
pub async fn setup_test_db() -> TestDatabase {
    sqlx::any::install_default_drivers();

    let temp_file = format!("test_{}.db", Uuid::new_v4());
    let db_file = PathBuf::from(&temp_file);
    let db_url = format!("sqlite://{}?mode=rwc", temp_file);

    let db = Database::connect(&db_url).await.expect("...");

    // Run ALL migrations from migrations/sqlite directory
    run_migrations(&db).await;

    TestDatabase { db, db_file }
}

async fn run_migrations(db: &Database) {
    sqlx::migrate!("./migrations/sqlite")
        .run(db.pool())
        .await
        .expect("Failed to run migrations");
}
```

### 4. Adding Test Data

**Helper Pattern:**
```rust
// tests/helpers/user_helpers.rs
pub async fn create_test_user(db: &Database, email: &str) -> User {
    let user_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    // Only INSERT data - schema already exists from migrations
    sqlx::query(
        "INSERT INTO users (id, email, user_type, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&user_id)
    .bind(email)
    .bind("agent")
    .bind(&now)
    .bind(&now)
    .execute(db.pool())
    .await
    .expect("Failed to insert test user");

    User { id: user_id, email: email.to_string(), ... }
}
```

### 5. Automatic Cleanup

**TestDatabase automatically cleans up files on Drop:**
```rust
impl Drop for TestDatabase {
    fn drop(&mut self) {
        // Removes test_*.db, test_*.db-journal, test_*.db-wal, test_*.db-shm
        let _ = fs::remove_file(&self.db_file);
        // ... cleanup other files
    }
}
```

### 6. Test Pattern

**Standard test structure:**
```rust
#[tokio::test]
async fn test_some_feature() {
    // Setup - runs migrations
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create test data (INSERT only)
    let user = create_test_user(&db, "test@example.com").await;

    // Test business logic
    let result = some_service.do_something(&user.id).await;

    // Assert
    assert!(result.is_ok());

    // Cleanup (optional - Drop handles it automatically)
    teardown_test_db(test_db).await;
}
```

### 7. Migration Files

**All schema changes go in migrations/:**
```
migrations/
├── sqlite/
│   ├── 001_create_users.sql
│   ├── 002_seed_roles_permissions.sql
│   ├── 003_create_inboxes.sql
│   ├── ...
│   └── 026_seed_sla_permissions.sql
├── postgres/
│   └── ... (same structure)
└── mysql/
    └── ... (same structure)
```

### 8. When You Need Schema Changes

1. Create a new migration file
2. Test the migration runs successfully
3. Tests automatically pick up the new schema
4. No test code changes needed (unless you want to test the new feature)

### 9. Current Test Coverage

- ✅ 184 tests passing
- ✅ All tests use migrations
- ✅ Zero DDL statements in test code
- ✅ Automatic database cleanup
- ✅ All 27 migrations validated by tests

### 10. Migration Validation

Every test run validates:
- All migrations apply successfully
- Migrations are idempotent (can be re-run)
- Schema matches what the application expects
- Foreign keys and constraints work correctly
- Indexes are created properly

## Summary

**Remember:** Tests are NOT the place for schema definitions. Tests verify that the schema (defined in migrations) works correctly with your business logic.

If you find yourself writing DDL in tests, stop and create a migration instead!
