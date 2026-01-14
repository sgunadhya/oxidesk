# Password Reset API Documentation

**Feature**: 017-password-reset
**Base URL**: `/api/password-reset`
**Authentication**: None (public endpoints)

## Overview

The Password Reset API provides secure password recovery for agents who have forgotten their passwords. The system uses time-limited, single-use tokens sent via email to verify identity before allowing password changes.

### Security Features

- **Email Enumeration Prevention**: Same response for valid and invalid emails
- **Rate Limiting**: Maximum 5 requests per hour per email address
- **Token Security**: 32-character alphanumeric tokens with 190 bits of entropy
- **Time-Limited**: Tokens expire after 1 hour
- **Single-Use**: Tokens are invalidated after successful password reset
- **Session Destruction**: All active sessions are destroyed on password reset
- **Password Complexity**: Enforced 10-72 character passwords with mixed complexity

---

## Endpoints

### 1. Request Password Reset

Initiates the password reset flow by generating a secure token and sending it via email.

**Endpoint**: `POST /api/password-reset/request`

**Request Headers**:
```
Content-Type: application/json
```

**Request Body**:
```json
{
  "email": "alice@example.com"
}
```

**Request Schema**:
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| email | string | Yes | Email address of the agent |

**Success Response** (200 OK):
```json
{
  "message": "If an account exists with that email, you will receive a password reset link."
}
```

**Note**: The same success response is returned regardless of whether the email exists in the system. This prevents email enumeration attacks.

**Error Responses**:

**400 Bad Request** - Invalid email format:
```json
{
  "error": "Invalid email format"
}
```

**429 Too Many Requests** - Rate limit exceeded:
```json
{
  "error": "Too many password reset requests. Please try again later."
}
```

**Example Request**:
```bash
curl -X POST http://localhost:3000/api/password-reset/request \
  -H "Content-Type: application/json" \
  -d '{
    "email": "alice@example.com"
  }'
```

**Side Effects** (if email exists):
- Generates 32-character alphanumeric reset token
- Stores token in database with 1-hour expiry
- Invalidates any previous unused tokens for this user
- Sends email with reset link to the agent
- Logs the reset request event

---

### 2. Reset Password

Completes the password reset flow using a valid token from the email.

**Endpoint**: `POST /api/password-reset/reset`

**Request Headers**:
```
Content-Type: application/json
```

**Request Body**:
```json
{
  "token": "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6",
  "new_password": "NewSecureP@ssw0rd2024"
}
```

**Request Schema**:
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| token | string | Yes | 32-character reset token from email |
| new_password | string | Yes | New password (must meet complexity requirements) |

**Password Complexity Requirements**:
- Length: 10-72 characters
- At least 1 uppercase letter (A-Z)
- At least 1 lowercase letter (a-z)
- At least 1 digit (0-9)
- At least 1 special character (!@#$%^&*()_+-=[]{}|;:,.<>?)

**Success Response** (200 OK):
```json
{
  "message": "Password has been reset successfully. Please log in with your new password."
}
```

**Error Responses**:

**400 Bad Request** - Invalid token format:
```json
{
  "error": "Invalid token format"
}
```

**400 Bad Request** - Invalid or expired token:
```json
{
  "error": "Invalid or expired reset token"
}
```

This error is returned when:
- Token does not exist in database
- Token has already been used
- Token has expired (> 1 hour old)

**400 Bad Request** - Weak password:
```json
{
  "error": "Password must be 10-72 characters long"
}
```

Or:
```json
{
  "error": "Password must contain at least one uppercase letter"
}
```

**Example Request**:
```bash
curl -X POST http://localhost:3000/api/password-reset/reset \
  -H "Content-Type: application/json" \
  -d '{
    "token": "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6",
    "new_password": "NewSecureP@ssw0rd2024"
  }'
```

**Side Effects** (on success):
- Agent password is updated (hashed with Argon2id)
- Reset token is marked as used
- All existing sessions for the agent are destroyed
- Agent must re-authenticate with new password
- Password reset success event is logged

---

## Complete Flow Example

### Step 1: Agent Requests Password Reset

```bash
curl -X POST http://localhost:3000/api/password-reset/request \
  -H "Content-Type: application/json" \
  -d '{"email": "alice@example.com"}'
```

Response:
```json
{
  "message": "If an account exists with that email, you will receive a password reset link."
}
```

### Step 2: Agent Receives Email

Email content:
```
Subject: Password Reset Request

You requested a password reset for your Oxidesk account.

Click the link below to reset your password:
https://app.example.com/reset-password?token=a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6

This link will expire in 1 hour.

If you did not request a password reset, please ignore this email.
```

### Step 3: Agent Clicks Link and Submits New Password

```bash
curl -X POST http://localhost:3000/api/password-reset/reset \
  -H "Content-Type: application/json" \
  -d '{
    "token": "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6",
    "new_password": "MyNewSecureP@ssw0rd2024"
  }'
```

Response:
```json
{
  "message": "Password has been reset successfully. Please log in with your new password."
}
```

### Step 4: Agent Logs In with New Password

```bash
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "alice@example.com",
    "password": "MyNewSecureP@ssw0rd2024"
  }'
```

---

## Rate Limiting

Password reset requests are rate-limited to prevent abuse:

- **Limit**: 5 requests per hour per email address
- **Window**: Rolling 1-hour window
- **Response**: 429 Too Many Requests (with error message)
- **Reset**: Counter resets 1 hour after first request

**Configuration**:
```env
PASSWORD_RESET_RATE_LIMIT=5  # Max requests per hour
```

---

## Token Lifecycle

```
[Generated] → [Sent via Email] → [Valid for 1 hour] → [Used or Expired]
     ↓              ↓                   ↓                    ↓
  Stored in      Email         Token can be      Token invalidated
  database     delivered      used to reset         (or deleted)
                              password once
```

**Token States**:
1. **Active**: Token is valid and can be used (unused, not expired)
2. **Used**: Token has been consumed for a password reset
3. **Expired**: Token is older than 1 hour (automatically rejected)
4. **Invalidated**: Token was superseded by a newer reset request

---

## Error Handling

### Client Error Scenarios

| Scenario | HTTP Status | Error Message |
|----------|-------------|---------------|
| Invalid email format | 400 | "Invalid email format" |
| Rate limit exceeded | 429 | "Too many password reset requests..." |
| Invalid token format | 400 | "Invalid token format" |
| Token doesn't exist | 400 | "Invalid or expired reset token" |
| Token already used | 400 | "Invalid or expired reset token" |
| Token expired | 400 | "Invalid or expired reset token" |
| Weak password | 400 | "Password must be 10-72 characters long" (or specific requirement) |

### Server Error Scenarios

| Scenario | HTTP Status | Error Message |
|----------|-------------|---------------|
| Database error | 500 | "An error occurred processing your request" |
| Email send failure | 200* | Generic success message (email failure is logged but doesn't fail the request) |

*Email sending is best-effort and does not block the response

---

## Configuration

### Environment Variables

```env
# SMTP Configuration (required for email delivery)
SMTP_HOST=smtp.example.com
SMTP_PORT=587
SMTP_USERNAME=noreply@example.com
SMTP_PASSWORD=smtp_password_here
SMTP_FROM_EMAIL=noreply@example.com
SMTP_FROM_NAME=Oxidesk Support

# Password Reset Settings
RESET_PASSWORD_BASE_URL=http://localhost:3000
PASSWORD_RESET_TOKEN_EXPIRY=3600  # seconds (1 hour)
PASSWORD_RESET_RATE_LIMIT=5       # requests per hour per email
```

---

## Security Considerations

### Email Enumeration Prevention

The API returns the same success message for both valid and invalid email addresses. This prevents attackers from discovering which emails are registered in the system.

**Example**:
- Request for `alice@example.com` (exists): "If an account exists..."
- Request for `nonexistent@example.com`: "If an account exists..." (same message)

### Token Security

- **Entropy**: 190 bits (62^32 possible tokens)
- **Format**: Alphanumeric only [a-zA-Z0-9]
- **Generation**: Cryptographically secure random generator
- **Storage**: Plain text in database (time-limited, single-use)
- **Transmission**: Only via HTTPS in production

### Session Security

After a successful password reset:
- All existing sessions are destroyed
- Agent must re-authenticate with new password
- This prevents session hijacking if password was compromised

---

## Testing

### Manual Testing Checklist

- [ ] Request reset with valid agent email
- [ ] Request reset with non-existent email (same response)
- [ ] Verify email is received with reset link
- [ ] Reset password with valid token
- [ ] Verify can log in with new password
- [ ] Attempt to reuse token (should fail)
- [ ] Attempt reset with expired token (should fail)
- [ ] Test rate limiting (6th request should fail)
- [ ] Verify all sessions destroyed after reset

### Example Test Cases

See the contract specifications in `/specs/017-password-reset/contracts/` for detailed test scenarios including edge cases.

---

## Logging

All password reset operations are logged with structured logging:

**Request Events**:
```
INFO: Password reset requested for email: alice@example.com (user_id: xxx)
INFO: Password reset requested for non-existent email: unknown@example.com
```

**Success Events**:
```
INFO: Password reset successful for user_id: xxx, sessions_destroyed: 3
```

**Error Events**:
```
ERROR: Failed to send password reset email to alice@example.com: <error>
```

**Rate Limit Events**:
```
WARN: Rate limit exceeded for email: alice@example.com
```

---

## Support

For implementation details, see:
- [Feature Specification](/specs/017-password-reset/spec.md)
- [API Contracts](/specs/017-password-reset/contracts/)
- [Data Model](/specs/017-password-reset/data-model.md)
- [Quickstart Guide](/specs/017-password-reset/quickstart.md)
