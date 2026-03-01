use std::env;

pub struct Config {
    pub database_url: String,
    pub sentry_dsn: Option<String>,
    pub environment: String,
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

        Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            sentry_dsn: env::var("SENTRY_DSN").ok(),
            environment: app_env,
        }
    }
}
