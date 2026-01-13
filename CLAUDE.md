# oxidesk Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-01-13 (010-macro-system)

## Active Technologies

- Rust 1.75+ + Axum 0.7 (web framework), sqlx 0.7 (database), argon2 0.5 (password hashing), tower 0.4 (middleware), serde 1.0 (serialization) (001-user-management)
- serde_json 1.0 (JSON serialization for rule conditions/actions), tokio broadcast channels (event bus for automation triggers) (009-automation-rule-engine)
- regex 1.10 (variable substitution in macro templates), JSON action queuing (reuses action types from automation engine) (010-macro-system)

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

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
