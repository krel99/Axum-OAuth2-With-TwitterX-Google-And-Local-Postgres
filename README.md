# OAuth2 with Axum

A working implementation of OAuth2 authentication using Rust's Axum web framework with Google and Twitter providers.

## What You'll Learn

- OAuth2 flow implementation with multiple providers (Google & Twitter)
- Session management with PostgreSQL and secure cookies
- Protected routes using middleware
- PKCE flow for enhanced security (Twitter)
- User profile extraction from OAuth providers

## Setup

### 1. Start PostgreSQL

```bash
docker-compose up -d
```

### 2. Configure OAuth Providers

**Google:**

1. Create OAuth 2.0 credentials at [Google Cloud Console](https://console.cloud.google.com/)
2. Add redirect URI: `http://localhost:8000/api/auth/google_callback`

**Twitter:**

1. Create OAuth 2.0 app at [Twitter Developer Portal](https://developer.twitter.com/)
2. Add redirect URL: `http://localhost:8000/api/auth/twitter_callback`

### 3. Set Environment Variables

Create `.env` file:

```env
GOOGLE_OAUTH_CLIENT_ID=your_google_client_id
GOOGLE_OAUTH_CLIENT_SECRET=your_google_client_secret
TWITTER_OAUTH_CLIENT_ID=your_twitter_client_id
TWITTER_OAUTH_CLIENT_SECRET=your_twitter_client_secret
DATABASE_URL=postgres://postgres:password@localhost/oauth_db
```

### 4. Run

```bash
cargo run
```

Navigate to `http://localhost:8000` and try logging in with Google or Twitter.

## Endpoints

- `/` - Home page with login options
- `/login` - Login page
- `/protected` - Protected area (requires authentication)
- `/protected/profile` - User profile
- `/api/auth/logout` - Logout

## Project Structure

```
src/
├── main.rs              # Application entry
├── config/             # Router configuration
├── handlers/           # Request handlers
├── middleware/         # Auth middleware
├── oauth/              # OAuth provider logic
├── services/           # Business logic
└── state/              # Application state
```
