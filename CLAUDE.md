# oxidesk Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-01-14 (016-user-creation)

## Active Technologies

- Rust 1.75+ + Axum 0.7 (web framework), sqlx 0.7 (database), argon2 0.5 (password hashing), tower 0.4 (middleware), serde 1.0 (serialization) (001-user-management)
- serde_json 1.0 (JSON serialization for rule conditions/actions), tokio broadcast channels (event bus for automation triggers) (009-automation-rule-engine)
- regex 1.10 (variable substitution in macro templates), JSON action queuing (reuses action types from automation engine) (010-macro-system)
- Axum SSE (Server-Sent Events for real-time push), regex 1.10 (@mention parsing), tokio::time intervals (scheduled cleanup), Arc<Mutex<HashMap>> (SSE connection management) (011-notification-system)
- reqwest 0.11 (async HTTP client with rustls-tls), hmac 0.12 + sha2 0.10 (HMAC-SHA256 signing), hex 0.4 (signature encoding), exponential backoff retry (1min, 2min, 4min, 8min, 16min) (012-webhook-system)
- bcrypt 0.15 (secret hashing with cost factor 12), rand 0.8 (cryptographic random generation for API keys/secrets), Axum middleware (API key authentication via X-API-Key/X-API-Secret headers or HTTP Basic Auth) (015-api-key-auth)
- regex 1.10 (email display name parsing for contact creation), rand 0.8 (16-character random password generation with mixed complexity), partial unique indexes (email uniqueness per user type), database transactions (atomic user + agent/contact + channel creation) (016-user-creation)

## Project Structure

```text
backend/
frontend/
tests/
```

## Commands

cargo test [ONLY COMMANDS FOR ACTIVE TECHNOLOGIES][ONLY COMMANDS FOR ACTIVE TECHNOLOGIES] cargo clippy

## Code Style

Rust 1.75+: Follow standard conventions

## Recent Changes

- 001-user-management: Added Rust 1.75+ + Axum 0.7 (web framework), sqlx 0.7 (database), argon2 0.5 (password hashing), tower 0.4 (middleware), serde 1.0 (serialization)
- 007-sla-lifecycle: Added SLA policy management, applied SLA tracking, SLA event lifecycle with breach detection (chrono for duration parsing, background workers for periodic breach checks)
- 009-automation-rule-engine: Added automation rule engine with event-driven condition evaluation and action execution (JSON-based rule storage, tokio broadcast for event subscription, cascade depth limiting)
- 010-macro-system: Added macro system with message templates, variable substitution, and action queuing (regex-based variable replacement, access control with all/restricted levels, application history logging)
- 011-notification-system: Added in-app notification system for agent alerts (assignment and @mention notifications, SSE for real-time delivery, automated cleanup with 30-day retention, unread tracking)
- 012-webhook-system: Added webhook system for external integrations (event subscription model, HMAC-SHA256 payload signing, HTTP delivery with exponential backoff retry, test webhooks, delivery logging)
- 015-api-key-auth: Added API key authentication for programmatic access (32-char API keys, 64-char secrets with bcrypt hashing, authentication via custom headers or HTTP Basic Auth, immediate revocation support)
- 016-user-creation: Added user creation flows (admin creates agents with random passwords, automatic contact creation from messages, email uniqueness per user type with partial indexes, display name parsing from email headers)

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
