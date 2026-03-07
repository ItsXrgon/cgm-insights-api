use std::env;

pub struct Config {
    pub database_url: String,
    pub sentry_dsn: Option<String>,
    pub sentry_release: Option<String>,
    pub environment: String,
    pub sentry_traces_sample_rate: f32,
    /// Background CGM sync interval in seconds (default: 36000 = 10 hours)
    pub sync_interval_secs: u64,
    /// HTTP port (default: 3000)
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        // Get APP_ENV first to know which file to load
        let app_env = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());

        // Load the environment-specific file if it exists, otherwise fall back to .env
        let env_file = format!(".env.{}", app_env);
        if dotenvy::from_filename(&env_file).is_err() {
            dotenvy::dotenv().ok();
        }

        let sync_interval_secs = env::var("SYNC_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(36000);

        let port = env::var("PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3000);

        let sentry_traces_sample_rate = env::var("SENTRY_TRACES_SAMPLE_RATE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1.0);

        Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            sentry_dsn: env::var("SENTRY_DSN").ok(),
            sentry_release: env::var("SENTRY_RELEASE").ok(),
            environment: app_env,
            sentry_traces_sample_rate,
            sync_interval_secs,
            port,
        }
    }
}
