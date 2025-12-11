# OAuth2 with Axum - Google Authentication

A complete OAuth2 authentication implementation using Axum web framework and Google as the OAuth provider, with protected routes and session management.

## Features

- Google OAuth2 authentication
- Protected routes that require authentication
- Session management with PostgreSQL
- Secure cookie-based sessions
- User profile storage
- Logout functionality

## Prerequisites

- Rust (latest stable version)
- PostgreSQL database
- Google Cloud Console account for OAuth2 credentials

## Setup

### 1. Database Setup

You have two options for setting up PostgreSQL:

#### Option A: Using Docker (Recommended)

```bash
# Start PostgreSQL with Docker Compose
docker-compose up -d

# The database 'oauth_db' will be created automatically
# PostgreSQL will be available on localhost:5432
# pgAdmin (optional) will be available on http://localhost:5050
```

#### Option B: Manual Setup

```bash
# Connect to PostgreSQL
psql -U postgres

# Create database
CREATE DATABASE oauth_db;

# Exit psql
\q
```

### 2. Google OAuth2 Setup

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project or select an existing one
3. Enable Google+ API
4. Go to "Credentials" and create OAuth 2.0 Client ID
5. Configure the OAuth consent screen
6. Set the following:
   - **Authorized JavaScript origins:**
     - `http://localhost`
     - `http://localhost:8000`
   - **Authorized redirect URIs:**
     - `http://localhost:8000/api/auth/google_callback`
     - `http://localhost/api/auth/google_callback`

### 3. Environment Variables

Create a `.env` file in the project root with your credentials:

```env
GOOGLE_OAUTH_CLIENT_ID=your_google_client_id_here
GOOGLE_OAUTH_CLIENT_SECRET=your_google_client_secret_here
DATABASE_URL=postgres://postgres:password@localhost/oauth_db
```

Replace the values with your actual Google OAuth credentials and database connection string.

### 4. Install Dependencies

```bash
# Install Rust dependencies
cargo build
```

## Running the Application

### Quick Start (Recommended)

Use the provided run script which handles database setup and application startup:

```bash
./run.sh
```

This script will:

- Check for required dependencies
- Start PostgreSQL with Docker (if available)
- Build and run the application
- Display available endpoints

### Manual Start

1. Make sure PostgreSQL is running:
   - If using Docker: `docker-compose up -d`
   - If manual setup: ensure your PostgreSQL service is running
2. Run the application:

```bash
cargo run
```

The server will start on `http://localhost:8000`

## Testing the Authentication Flow

### Manual Testing

1. Open your browser and navigate to `http://localhost:8000`
2. Click "Sign in with Google" or visit the interactive demo at `/static/index.html`
3. You'll be redirected to Google's login page
4. After successful authentication, you'll be redirected back to the protected area
5. Try accessing the protected routes:
   - `/protected` - Main protected area
   - `/protected/profile` - User profile page

### Automated Testing

Run the test script to verify all endpoints:

```bash
./test_oauth.sh
```

This will test:

- Public endpoints availability
- Protected endpoints authentication requirements
- Health check functionality
- Static file serving
- Security headers

## API Endpoints

### Public Endpoints

- `GET /` - Homepage with interactive demo
- `GET /login` - Login page with Google OAuth link
- `GET /health` - Health check endpoint (returns JSON)
- `GET /static/index.html` - Enhanced interactive demo page

### Authentication Endpoints

- `GET /api/auth/google_callback` - OAuth callback endpoint (handled automatically)
- `GET /api/auth/logout` - Logout endpoint

### Protected Endpoints

- `GET /protected` - Protected area (requires authentication)
- `GET /protected/profile` - User profile (requires authentication)

## Project Structure

```
oauth_axum/
├── src/
│   ├── main.rs        # Main application with routes and OAuth logic
│   └── errors.rs      # Error handling
├── migrations/
│   └── *.sql         # Database migrations
├── static/
│   └── index.html    # Interactive demo interface
├── Cargo.toml        # Rust dependencies
├── docker-compose.yml # Docker setup for PostgreSQL
├── run.sh           # Quick start script
├── test_oauth.sh    # API testing script
├── .env             # Environment variables (not committed)
├── .gitignore       # Git ignore rules
└── README.md        # This file
```

## Features

### Core Features

- ✅ Google OAuth2 authentication
- ✅ Session management with PostgreSQL
- ✅ Protected routes with middleware
- ✅ Encrypted cookie sessions
- ✅ User profile storage
- ✅ Automatic session expiration
- ✅ Health check endpoint
- ✅ Interactive demo UI

### Developer Features

- ✅ Quick start script (`run.sh`)
- ✅ Automated testing suite (`test_oauth.sh`)
- ✅ Docker Compose for database
- ✅ Environment-based configuration
- ✅ Comprehensive error handling
- ✅ Logging and debugging support

## Security Notes

- The `.env` file contains sensitive credentials and should never be committed to version control
- Sessions are stored in PostgreSQL with expiration times
- Cookies are encrypted using a secure key generated at startup
- All authentication cookies are HTTP-only to prevent XSS attacks
- CORS is configured for development (adjust for production)

## Troubleshooting

### Database Connection Issues

- If using Docker:
  - Check if containers are running: `docker-compose ps`
  - View logs: `docker-compose logs postgres`
  - Restart if needed: `docker-compose restart`
- If using manual setup:
  - Ensure PostgreSQL is running: `sudo systemctl status postgresql`
- Check your DATABASE_URL in the `.env` file
- Make sure the database `oauth_db` exists

### OAuth Issues

- Verify your Google OAuth credentials are correct
- Check that redirect URIs match exactly in Google Console and the application
- Ensure you're accessing the app via `http://localhost:8000` (not `127.0.0.1`)

### Port Already in Use

If port 8000 is already in use, you can change it in `main.rs`:

```rust
let listener = tokio::net::TcpListener::bind("0.0.0.0:YOUR_PORT")
```

Remember to update the redirect URIs in Google Console accordingly.

### Stopping the Application

If using Docker for the database:

```bash
# Stop the database containers
docker-compose down

# To also remove the data volumes (this will delete all data):
docker-compose down -v
```

## License

This project is provided as-is for educational purposes.
