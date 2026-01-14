# Oxidesk

> **A modern, intelligent customer support platform built for teams that care about response times, service quality, and customer satisfaction.**

Oxidesk helps your support team manage customer conversations efficiently with powerful automation, smart assignment, SLA tracking, and real-time collaboration tools.

---

## Why Oxidesk?

Traditional helpdesk software is either too complex or too simple. Oxidesk strikes the perfect balance:

- **Smart Conversation Management** - Email integration, automatic threading, and reference numbers keep conversations organized
- **Intelligent Assignment** - Automatic routing based on availability, workload, and team capacity
- **SLA Tracking** - Set response time targets and track performance with business hours and holiday support
- **Team Collaboration** - Real-time notifications, conversation tagging, and role-based access control
- **Automation** - Create rules to automatically assign, tag, prioritize, and respond to conversations
- **Multi-Channel Support** - Start with email, expand to other channels as you grow

---

## Core Features

### 📧 Email Integration
- **IMAP email receiving** - Automatically fetch emails from your support inbox
- **Smart threading** - Replies are automatically matched to existing conversations via reference numbers
- **Attachment handling** - Store and retrieve email attachments securely
- **Send from conversation** - Reply directly from the conversation view

### 🎯 Conversation Management
- **Status tracking** - Open, Snoozed, Resolved, Closed states with automatic transitions
- **Priority levels** - Low, Normal, High, Urgent with visual indicators
- **Tagging system** - Organize conversations with custom tags and colors
- **Smart filtering** - Find conversations by status, assignee, priority, or tags
- **Reference numbers** - Each conversation gets a unique #REF number for easy tracking

### 👥 Team Management
- **Role-based access** - Create custom roles with granular permissions
- **Team assignments** - Assign conversations to individuals or entire teams
- **Availability tracking** - Agents can set their status (Available, Busy, Away)
- **Automatic unassignment** - When agents go away, their conversations return to the queue
- **Workload balancing** - See conversation counts per agent at a glance

### ⚡ Smart Assignment
- **Self-assignment** - Agents can claim unassigned conversations
- **Manual assignment** - Assign to specific agents or teams
- **Auto-assignment** - Configure rules to automatically route conversations
- **Concurrent protection** - Built-in race condition handling prevents double assignment
- **Assignment history** - Full audit trail of who handled each conversation

### 📊 SLA Management
- **Response time tracking** - First response, next response, and resolution SLAs
- **Business hours support** - Define working hours by day of week
- **Holiday calendar** - Exclude holidays from SLA calculations
- **Weekend exclusion** - Automatically skip weekends in deadline calculations
- **Breach detection** - Get notified when SLAs are at risk or breached
- **Team-based SLAs** - Different SLA policies for different teams

### 🤖 Automation Rules
- **Event-based triggers** - React to conversation status changes, messages, assignments
- **Conditional logic** - Complex conditions with AND/OR operators
- **Multiple actions** - Set status, assign, tag, set priority in one rule
- **Priority ordering** - Control rule execution order
- **Cascade prevention** - Automatic infinite loop detection
- **Audit logging** - See exactly when and why rules fired

### 🔐 Security & Access Control
- **Role-based permissions** - 60+ granular permissions for fine-grained control
- **Password reset flow** - Secure email-based password reset with rate limiting
- **Session management** - Automatic session expiration and security
- **API key support** - Authenticate API requests without exposing passwords
- **OIDC integration** - Single sign-on with Google and other providers
- **Audit trails** - Track all security-relevant actions

### 🔔 Notifications
- **Real-time updates** - WebSocket notifications for instant updates
- **Assignment alerts** - Get notified when conversations are assigned to you
- **SLA warnings** - Alerts before SLAs breach
- **Database persistence** - Never miss a notification, even offline
- **Mark as read** - Keep track of what you've seen

### 💬 Macros (Saved Replies)
- **Text templates** - Save common responses for quick replies
- **Team sharing** - Share macros with your entire team
- **Personal macros** - Keep private macros for your own use
- **Quick access** - Insert macros with a single click

### 📈 Reporting & Analytics
- **Conversation metrics** - Track volume, response times, resolution times
- **Agent performance** - See individual and team statistics
- **SLA compliance** - Monitor SLA achievement rates
- **Tag analytics** - Identify common conversation topics
- **Priority distribution** - Understand your workload composition

---

## Quick Start

### 1. Install and Run

```bash
# Clone the repository
git clone https://github.com/your-org/oxidesk.git
cd oxidesk

# Create configuration file
cp .env.example .env

# Edit .env with your settings
nano .env

# Build and run
cargo build --release
cargo run --release
```

### 2. First Login

1. Open http://localhost:8080/login
2. Login with your admin credentials from `.env`:
   - Email: Your `ADMIN_EMAIL`
   - Password: Your `ADMIN_PASSWORD`

### 3. Set Up Your Team

1. **Create roles** - Go to Settings → Roles, create roles like "Support Agent", "Team Lead"
2. **Invite agents** - Go to Team → Agents, add your support team members
3. **Create teams** - Organize agents into teams (Sales Support, Technical Support, etc.)
4. **Configure inboxes** - Set up email integration for your support inbox

### 4. Configure SLAs

1. **Create SLA policy** - Go to Settings → SLA Policies
   - Set first response time (e.g., 4 hours)
   - Set resolution time (e.g., 24 hours)
2. **Add business hours** - Define your working hours (Mon-Fri 9am-5pm)
3. **Add holidays** - Configure your company holidays
4. **Assign to teams** - Link SLA policies to teams

### 5. Set Up Automation

1. **Create assignment rules** - Auto-assign based on tags, priority, or keywords
2. **Create tagging rules** - Auto-tag conversations based on content
3. **Create priority rules** - Auto-escalate based on keywords or contact history

---

## How To...

<details>
<summary><b>Handle incoming customer emails</b></summary>

1. **Automatic receipt** - Oxidesk polls your IMAP inbox every 60 seconds
2. **Conversation creation** - New emails become new conversations
3. **Smart threading** - Replies are matched to existing conversations by reference number
4. **Auto-assignment** - If you have rules configured, conversations are automatically assigned
5. **Notifications** - Assigned agents receive real-time notifications

</details>

<details>
<summary><b>Respond to a customer</b></summary>

1. Open the conversation from your dashboard or inbox
2. Type your response in the message box
3. Click "Send" - the email is sent via SMTP and recorded in the conversation
4. The conversation updates automatically with your response
5. Customer receives email with unique reference number for tracking

</details>

<details>
<summary><b>Track SLA compliance</b></summary>

1. **View SLA status** - Each conversation shows SLA deadlines
2. **Color coding** - Green (met), Yellow (at risk), Red (breached)
3. **Filter by SLA status** - Find all conversations at risk of breach
4. **Reports** - See team-wide SLA compliance metrics
5. **Notifications** - Get alerted before deadlines are breached

</details>

<details>
<summary><b>Create automation rules</b></summary>

1. Go to Settings → Automation Rules
2. Click "New Rule"
3. **Name your rule** - e.g., "Auto-assign billing questions"
4. **Set trigger** - Choose event (Message Created, Status Changed, etc.)
5. **Add conditions** - e.g., "Subject contains 'billing' AND Priority is High"
6. **Choose action** - e.g., "Assign to team: Billing Support"
7. Save and enable

Rules fire automatically when conditions match!

</details>

<details>
<summary><b>Set up team workflows</b></summary>

1. **Create teams** - Settings → Teams (e.g., "Tier 1", "Tier 2", "Billing")
2. **Add members** - Assign agents to teams
3. **Set SLA policies** - Each team can have different response time targets
4. **Configure business hours** - Teams can have different working schedules
5. **Create routing rules** - Auto-route conversations to the right team

</details>

<details>
<summary><b>Manage agent availability</b></summary>

Agents can set their status:
- **Available** - Ready to take new assignments
- **Busy** - Working but not taking new work
- **Away** - Out of office, conversations auto-unassigned

Status changes trigger automatic workflow actions!

</details>

---

## Configuration

### Database Setup

Choose your database (SQLite for development, PostgreSQL/MySQL for production):

```env
# SQLite (easiest to start)
DATABASE_URL=sqlite://oxidesk.db

# PostgreSQL (recommended for production)
DATABASE_URL=postgres://user:password@localhost/oxidesk

# MySQL (also supported)
DATABASE_URL=mysql://user:password@localhost/oxidesk
```

### Email Integration

Configure IMAP (receiving) and SMTP (sending):

```env
# IMAP - Receive emails
IMAP_HOST=imap.gmail.com
IMAP_PORT=993
IMAP_USERNAME=support@yourcompany.com
IMAP_PASSWORD=your-app-password
IMAP_USE_TLS=true

# SMTP - Send emails
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USERNAME=support@yourcompany.com
SMTP_PASSWORD=your-app-password
SMTP_FROM_EMAIL=support@yourcompany.com
SMTP_FROM_NAME=Your Company Support
```

### Authentication Options

Oxidesk supports multiple authentication methods:

```env
# Password-based (default)
SESSION_DURATION_HOURS=24

# OIDC/OAuth (Google, etc.)
OIDC_GOOGLE_CLIENT_ID=your-client-id
OIDC_GOOGLE_CLIENT_SECRET=your-client-secret
OIDC_GOOGLE_REDIRECT_URI=http://localhost:8080/auth/oidc/google/callback

# API Keys (for integrations)
# Generated per-agent via the UI
```

### Performance Tuning

```env
# Session cleanup
SESSION_CLEANUP_INTERVAL_HOURS=1

# Email polling
EMAIL_POLL_INTERVAL_SECONDS=60

# SLA breach checking
SLA_CHECK_INTERVAL_SECONDS=300

# Rate limiting
PASSWORD_RESET_RATE_LIMIT=5  # per hour
```

---

## API Access

Oxidesk provides a complete REST API for integrations:

```bash
# Get API token
curl -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "your@email.com", "password": "yourpassword"}'

# Use token in requests
curl http://localhost:8080/api/conversations \
  -H "Authorization: Bearer <your-token>"
```

### Key API Endpoints

- `GET /api/conversations` - List conversations
- `POST /api/conversations/:id/messages` - Send a message
- `PATCH /api/conversations/:id/status` - Update status
- `POST /api/conversations/:id/assign` - Assign conversation
- `GET /api/agents` - List team members
- `GET /api/sla/policies` - List SLA policies
- `POST /api/automation/rules` - Create automation rule

See full API documentation at `/api/docs` when running.

---

## Deployment

### Docker Deployment

```dockerfile
# Use official image
docker pull ghcr.io/your-org/oxidesk:latest

# Run with environment config
docker run -d \
  --name oxidesk \
  -p 8080:8080 \
  -e DATABASE_URL=postgres://user:pass@db/oxidesk \
  -e ADMIN_EMAIL=admin@example.com \
  -e ADMIN_PASSWORD=SecurePassword123! \
  ghcr.io/your-org/oxidesk:latest
```

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: oxidesk
spec:
  replicas: 3
  template:
    spec:
      containers:
      - name: oxidesk
        image: ghcr.io/your-org/oxidesk:latest
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: oxidesk-secrets
              key: database-url
```

### Reverse Proxy (nginx)

```nginx
server {
    listen 443 ssl;
    server_name support.yourcompany.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location / {
        proxy_pass http://localhost:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

---

## Support & Community

- **Issues**: Report bugs at [GitHub Issues](https://github.com/your-org/oxidesk/issues)
- **Discussions**: Ask questions in [GitHub Discussions](https://github.com/your-org/oxidesk/discussions)
- **Documentation**: Full docs at [docs.oxidesk.io](https://docs.oxidesk.io)
- **Email**: support@oxidesk.io

---

## Roadmap

- ✅ Core conversation management
- ✅ Email integration (IMAP/SMTP)
- ✅ SLA tracking with business hours
- ✅ Automation rules engine
- ✅ Team collaboration
- 🚧 Knowledge base integration
- 🚧 Chat widget for websites
- 🚧 Mobile apps (iOS/Android)
- 📋 Social media integration
- 📋 AI-powered response suggestions
- 📋 Customer satisfaction surveys

---

## License

MIT License - see [LICENSE](LICENSE) for details

---

<details>
<summary><h2>📚 Technical Details (For Developers)</h2></summary>

### Technology Stack

- **Language**: Rust 1.75+
- **Web Framework**: Axum 0.7
- **Database**: sqlx 0.7 (SQLite, PostgreSQL, MySQL)
- **Frontend**: HTMX + Server-rendered HTML (Askama templates)
- **Authentication**: Argon2id password hashing, session-based auth
- **Real-time**: WebSocket notifications
- **Email**: async-imap (receive), lettre (send)

### Architecture

```
┌─────────────────┐
│  Web Handlers   │  ← HTMX templates, thin HTTP layer
└────────┬────────┘
         │
┌────────▼────────┐
│  API Handlers   │  ← JSON responses, thin HTTP layer
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

**Key Principles:**
- Handlers are thin (no business logic)
- Services contain all business logic
- Database layer is pure CRUD operations
- Models are shared across layers

### Security Features

- **Argon2id**: Password hashing with m_cost=19456, t_cost=2, p_cost=1
- **Prepared Statements**: All SQL queries are parameterized
- **CSRF Protection**: Token-based CSRF validation
- **Session Security**: HttpOnly cookies, automatic expiration
- **Rate Limiting**: Password reset, API endpoints
- **Input Validation**: Email format, password complexity
- **Soft Deletes**: Data retention for audit trails

### Database Schema

Key tables:
- `users` - Agents and contacts (polymorphic)
- `conversations` - Customer support conversations
- `messages` - Conversation messages (inbound/outbound)
- `sla_policies` - Response time targets
- `applied_slas` - SLA tracking per conversation
- `automation_rules` - Business logic automation
- `teams` - Agent organization
- `holidays` - Non-working days for SLA calculation

### Performance

- **Database Connection Pooling**: sqlx connection pool
- **Async I/O**: Tokio async runtime
- **Efficient Queries**: Indexed lookups, pagination
- **Background Tasks**: Tokio tasks for email polling, SLA checking
- **Caching**: In-memory caching for frequently accessed data

### Testing

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test '*'

# All tests with logging
RUST_LOG=debug cargo test

# Coverage
cargo tarpaulin --out Html
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint (strict)
cargo clippy -- -D warnings

# Security audit
cargo audit

# Check dependencies
cargo outdated
```

### Development Setup

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install cargo-watch (hot reload)
cargo install cargo-watch

# Run with hot reload
cargo watch -x run

# Run tests on file change
cargo watch -x test
```

### Project Structure

```
src/
├── api/              # API handlers (REST endpoints)
├── web/              # Web handlers (HTMX views)
├── services/         # Business logic
│   ├── assignment_service.rs
│   ├── sla_service.rs
│   ├── automation_service.rs
│   └── ...
├── database/         # Data access layer
├── models/           # Data structures & DTOs
├── events/           # Event bus for pub/sub
├── utils/            # Shared utilities
└── main.rs           # Entry point

migrations/           # SQL migrations
├── sqlite/
├── postgres/
└── mysql/

templates/            # Askama HTML templates
tests/                # Integration tests
```

### Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Follow the code style (`cargo fmt`)
4. Add tests for new features
5. Ensure all tests pass (`cargo test`)
6. Commit your changes (`git commit -m 'Add amazing feature'`)
7. Push to the branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

**Code Guidelines:**
- Follow Rust idioms and best practices
- Write comprehensive tests
- Document public APIs with rustdoc
- Keep services focused and single-purpose
- Never put business logic in handlers
- Use the event bus for cross-service communication

</details>
