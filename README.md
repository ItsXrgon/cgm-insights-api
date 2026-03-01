# CGM Insights API

A Rust-based API for syncing and storing Continuous Glucose Monitor (CGM) data from multiple platforms (LibreLink Up, Dexcom).

## Features

- **Automated Multi-User Sync**: Fetches glucose readings every 5 minutes for all active user credentials.
- **Support for Multiple CGMs**: Users can store multiple CGM credentials (e.g., FreeStyle, Dexcom).
- **Active Credential Management**: Switch between different CGM sources or update credentials easily.
- **JWT Authentication**: Secure signup and login system to protect user data.
- **Sentry Error Tracking**: Full integration with Sentry for real-time error tracking, logging, and release management.
- **Environment Support**: Flexible configuration using `.env.development`, `.env.production`, or standard environment variables.
- **REST API**: 
    - `POST /api/auth/signup`: Register with initial CGM credentials.
    - `POST /api/auth/login`: Authenticate and receive a JWT.
    - `GET /api/glucose`: Retrieve your historical glucose readings.
    - `POST /api/sync`: Manually trigger a synchronization for your active CGM.
    - `GET /api/cgm`: Manage your CGM credentials.
    - `GET /health`: System health check (public).
- **Security**: 
    - **Rate Limiting**: Protects against brute-force and DDoS (per-IP).
    - **Security Headers**: NOSNIFF, HSTS, XSS protection, and frame denial.
    - **Authentication**: JWT-based protection for all `/api` endpoints.
    - **Timeouts**: Automated request cancellation after 30 seconds.
- **Efficient Storage**: Uses bulk inserts and database-level deduplication per user.

## Prerequisites

- Rust (latest stable)
- PostgreSQL

## Configuration

The application uses `dotenvy` to load configuration from environment files. It supports environment-specific files based on the `APP_ENV` variable.

### Environment Files
- `.env.development`: Loaded when `APP_ENV=development` (default).
- `.env.production`: Loaded when `APP_ENV=production`.
- `.env`: Fallback file if the specific environment file is missing.

### Environment Variables
Set the following variables in your `.env` file or environment:

| Variable | Description | Default |
|----------|-------------|---------|
| `APP_ENV` | Application environment (`development`, `production`) | `development` |
| `DATABASE_URL` | PostgreSQL connection string | (Required) |
| `JWT_SECRET` | Secret key for JWT signing | (Required) |
| `SENTRY_DSN` | Sentry project DSN (optional) | (None) |
| `RUST_LOG` | Logging level (`error`, `warn`, `info`, `debug`, `trace`) | `info` |

### Setting Up Sentry
To enable Sentry, simply provide the `SENTRY_DSN` environment variable. If omitted, Sentry initialization and its associated middlewares (tracing and HTTP layers) will be automatically disabled to avoid overhead.

## Getting Started

1. Clone the repository
2. Create your environment file: `cp .env.example .env.development`
3. Edit `.env.development` with your local settings.
4. Run the application: `cargo run`

To run in production mode:
```bash
APP_ENV=production cargo run
```

## Improvements Made

- **Optional Sentry Configuration**: Sentry is now fully optional. Tracing and HTTP middleware layers are only attached if `SENTRY_DSN` is provided. The release name is automatically determined from the package version in `Cargo.toml`.
- **Multi-Environment Support**: The application now dynamically loads `.env.development` or `.env.production` based on the `APP_ENV` variable.
- **User Authentication**: Implemented a full Signup/Login system with password hashing (Argon2) and JWT tokens.
- **Multi-Credential Support**: Users can now have multiple CGM credentials and choose which one is active.
- **Multi-Tenant Architecture**: Glucose readings are now linked to specific users, allowing the server to serve multiple users concurrently.
- **Scalable Syncing**: The background scheduler now iterates through all active credentials across all users.
- **Migration Support**: Updated database initialization to handle schema changes (like adding `user_id` to existing readings).
