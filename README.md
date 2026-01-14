# Oxidesk

Customer support helpdesk system built with Rust, Axum, and sqlx featuring clean architecture, RBAC, and modern HTMX frontend.

## Features

- **User Management**: Support for agents (support staff) and contacts (customers)
- **Role-Based Access Control**: Flexible permission system for agents
- **Multi-Database Support**: Works with SQLite, PostgreSQL, and MySQL from a single binary
- **Service Layer Architecture**: Clean separation of concerns (Handlers → Services → Database)
- **RESTful API**: Complete REST API with session-based authentication
- **HTMX Frontend**: Server-rendered HTML with HTMX for dynamic interactions
- **Secure by Default**: Argon2id password hashing, prepared statements, input validation
- **Session Management**: Automatic cleanup of expired sessions
- **Structured Logging**: Comprehensive logging for security and debugging

## Architecture

Oxidesk follows a strict layered architecture enforced by the project constitution:

```
┌─────────────────┐
│  Web Handlers   │  ← Thin HTTP layer (HTMX templates)
└────────┬────────┘
         │
┌────────▼────────┐
│  API Handlers   │  ← Thin HTTP layer (JSON responses)
└────────┬────────┘
         │
┌────────▼────────┐
│    Services     │  ← Business logic & validation
└────────┬────────┘
         │
┌────────▼────────┐
│    Database     │  ← Pure data access (CRUD)
└─────────────────┘
```

See [.speckit/constitution.md](.speckit/constitution.md) for full architecture guidelines.

## Quick Start

### Prerequisites

- Rust 1.75+
- SQLite (included) OR PostgreSQL 14+ OR MySQL 8.0+

### Environment Configuration

Create a `.env` file or set environment variables:

```env
# Database
DATABASE_URL=sqlite://oxidesk.db

# Server
HOST=127.0.0.1
PORT=8080

# Admin Account (created on first run)
ADMIN_EMAIL=admin@example.com
ADMIN_PASSWORD=SecurePassword123!

# Session
SESSION_DURATION_HOURS=24

# Logging
RUST_LOG=oxidesk=info,tower_http=info
```

### Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/your-org/oxidesk.git
   cd oxidesk
   ```

2. Build and run:
   ```bash
   cargo build --release
   cargo run --release
   ```

3. Access the application:
   - Web UI: http://localhost:8080/login
   - API: http://localhost:8080/api
   - Health: http://localhost:8080/health

Default admin credentials are set via `ADMIN_EMAIL` and `ADMIN_PASSWORD` environment variables.

## API Documentation

### Authentication

```bash
# Login (get Bearer token)
curl -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "admin@example.com", "password": "SecurePassword123!"}'

# Returns: {"token": "session-token-here"}

# Use token in subsequent requests
curl http://localhost:8080/api/agents \
  -H "Authorization: Bearer <token>"

# Get current session
GET /api/auth/session

# Logout
POST /api/auth/logout
```

### Agents API

```bash
# List agents (paginated)
GET /api/agents?page=1&per_page=20

# Create agent
POST /api/agents
{
  "email": "agent@example.com",
  "first_name": "John",
  "password": "SecurePassword123!",
  "role_ids": ["<role-id>"]
}

# Get agent by ID
GET /api/agents/:id

# Update agent
PATCH /api/agents/:id
{
  "first_name": "Jane",
  "role_ids": ["<role-id>"]
}

# Delete agent
DELETE /api/agents/:id

# Change password
POST /api/agents/:id/password
{
  "new_password": "NewPassword123!"
}
```

### Contacts API

```bash
# List contacts (paginated)
GET /api/contacts?page=1&per_page=20

# Create contact
POST /api/contacts
{
  "email": "contact@example.com",
  "first_name": "Alice",
  "channels": [
    {"inbox_id": "<inbox-id>", "email": "alice@example.com"}
  ]
}

# Get contact by ID
GET /api/contacts/:id

# Update contact
PATCH /api/contacts/:id
{
  "first_name": "Bob"
}

# Delete contact
DELETE /api/contacts/:id
```

### Roles & Permissions API

```bash
# List roles
GET /api/roles

# Create role
POST /api/roles
{
  "name": "Support Agent",
  "description": "Customer support role",
  "permission_ids": ["<permission-id>"]
}

# Get role by ID
GET /api/roles/:id

# Update role
PATCH /api/roles/:id
{
  "name": "Senior Support Agent",
  "permission_ids": ["<permission-id>"]
}

# Delete role
DELETE /api/roles/:id

# List all permissions
GET /api/permissions
```

### Users API (Generic)

```bash
# List all users (agents and contacts)
GET /api/users?type=agent  # Optional: filter by type

# Get user by ID
GET /api/users/:id

# Delete user by ID
DELETE /api/users/:id
```

## Web Interface

Access the HTMX-powered web UI at http://localhost:8080/login

### Routes

- `/login` - Login page
- `/dashboard` - Main dashboard with stats
- `/agents` - Agent management (list, delete)
- `/contacts` - Contact management (list, delete)
- `/roles` - Role management (list, delete)
- `/logout` - Logout (POST)

All management pages require authentication and appropriate permissions.

## Development

### Running Tests

```bash
# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test '*'

# All tests
cargo test

# With logging
RUST_LOG=debug cargo test
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint code
cargo clippy -- -D warnings

# Check compilation
cargo check

# Run in development mode with hot reload
cargo watch -x run
```

### Project Structure

```
src/
├── api/              # API handlers (thin)
│   ├── agents.rs
│   ├── contacts.rs
│   ├── roles.rs
│   ├── users.rs
│   └── auth.rs
├── web/              # Web handlers (thin)
│   └── mod.rs
├── services/         # Business logic (thick)
│   ├── agent_service.rs
│   ├── contact_service.rs
│   ├── role_service.rs
│   ├── auth.rs
│   └── email_validator.rs
├── database/         # Data access (CRUD only)
│   └── mod.rs
├── models/           # Data structures & DTOs
│   ├── user.rs
│   ├── role.rs
│   └── session.rs
├── config.rs         # Configuration
└── main.rs           # Entry point

templates/            # Askama templates
├── base.html         # Base layout
├── login.html
├── dashboard.html
├── agents.html
├── contacts.html
├── roles.html
└── partials/
    └── error.html    # Reusable error template

migrations/           # SQL migrations
├── sqlite/
├── postgres/
└── mysql/
```

## Security Features

- **Argon2id Password Hashing**: Industry-standard (m_cost=19456, t_cost=2, p_cost=1)
- **HttpOnly Cookies**: Session tokens secured from XSS attacks
- **Last Admin Protection**: Cannot delete the final admin user
- **Password Complexity**: Enforced 12-128 chars, uppercase, lowercase, digit, special char
- **Session Expiration**: Automatic hourly cleanup of expired sessions
- **Input Validation**: Email validation with TLD requirements
- **Per-Type Email Uniqueness**: Same email can be agent AND contact
- **SQL Injection Prevention**: All queries use parameterized statements

## Background Tasks

- **Session Cleanup**: Runs every hour to remove expired sessions from the database

## Logging

Structured logging via `tracing`:

```bash
# Set log level
RUST_LOG=oxidesk=debug,tower_http=debug cargo run
```

Key events logged:
- Login attempts (success/failure with reasons)
- Logout events
- Agent/Contact/Role deletions
- Permission denials
- Session cleanup statistics
- Admin protection violations
- Database operations

## Production Deployment

### Database Configuration

**SQLite** (Development):
```env
DATABASE_URL=sqlite://oxidesk.db
```

**PostgreSQL** (Production):
```env
DATABASE_URL=postgres://user:password@localhost/oxidesk
```

**MySQL** (Production):
```env
DATABASE_URL=mysql://user:password@localhost/oxidesk
```

### Environment Variables

```env
# Required
DATABASE_URL=<database-url>
ADMIN_EMAIL=<admin-email>
ADMIN_PASSWORD=<strong-password>

# Optional
HOST=0.0.0.0
PORT=8080
SESSION_DURATION_HOURS=24
RUST_LOG=oxidesk=info,tower_http=warn

# Password Reset / SMTP Email (required for password reset feature)
SMTP_HOST=smtp.example.com           # SMTP server hostname
SMTP_PORT=587                        # SMTP port (587 for TLS, 465 for SSL)
SMTP_USERNAME=noreply@example.com    # SMTP authentication username
SMTP_PASSWORD=smtp_password          # SMTP authentication password
SMTP_FROM_EMAIL=noreply@example.com  # Sender email address
SMTP_FROM_NAME=Oxidesk Support       # Sender name in email
RESET_PASSWORD_BASE_URL=http://localhost:3000  # Base URL for reset links
PASSWORD_RESET_TOKEN_EXPIRY=3600     # Token expiry in seconds (default: 1 hour)
PASSWORD_RESET_RATE_LIMIT=5          # Max requests per hour per email
```

### Build & Run

```bash
# Build for production
cargo build --release

# Run
./target/release/oxidesk
```

### Docker (Optional)

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/oxidesk /usr/local/bin/
CMD ["oxidesk"]
```

## Documentation

- [Feature Specification](specs/001-user-management/spec.md) - Requirements and user stories
- [Implementation Plan](specs/001-user-management/plan.md) - Technical architecture
- [Constitution](.speckit/constitution.md) - Architecture principles and code guidelines
- [Data Model](specs/001-user-management/data-model.md) - Database schema
- [API Contracts](specs/001-user-management/contracts/openapi.yaml) - OpenAPI specification
- [Quickstart Guide](specs/001-user-management/quickstart.md) - Setup and usage

## Contributing

1. Follow the architecture principles in `.speckit/constitution.md`
2. Write tests for all new features
3. Use the service layer for business logic
4. Never put HTML strings in code - use templates
5. Add structured logging for important operations

## License

MIT
