use anyhow::{bail, Context};
use std::env;
use std::str::FromStr;

/// Placeholder shipped in `.env.example`; never acceptable outside development.
const EXAMPLE_JWT_SECRET: &str = "your_jwt_secret_key_change_me_in_production";
const DEV_JWT_SECRET: &str = "development-only-insecure-jwt-secret";
const MIN_JWT_SECRET_LEN: usize = 32;
const DEFAULT_ALLOWED_ORIGIN: &str = "https://xrgon.com";

/// All runtime configuration, read once at startup. Nothing else reads the environment.
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    /// `APP_ENV`, default `development`
    pub environment: String,
    /// Only an explicit (or defaulted) `development` gets relaxed security defaults.
    pub is_development: bool,
    pub jwt_secret: String,
    /// CORS origins; `None` allows any origin (development only)
    pub allowed_origins: Option<Vec<String>>,
    pub sentry_dsn: Option<String>,
    pub sentry_release: Option<String>,
    pub sentry_traces_sample_rate: f32,
    /// Background sync interval in seconds (default 3600 = 1 hour)
    pub sync_interval_secs: u64,
    /// Minimum seconds between sync attempts for one credential (default 60)
    pub sync_cooldown_secs: u64,
    /// HTTP port (default 3000)
    pub port: u16,
}

impl Config {
    /// Load `.env.{APP_ENV}` (or `.env`) and read the environment.
    pub fn from_env() -> anyhow::Result<Self> {
        let app_env = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
        if dotenvy::from_filename(format!(".env.{}", app_env)).is_err() {
            dotenvy::dotenv().ok();
        }
        Self::from_lookup(|key| env::var(key).ok())
    }

    /// Build from any key lookup; `from_env` uses the process environment, tests a map.
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> anyhow::Result<Self> {
        let var = |key: &str| {
            get(key)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };

        let environment = var("APP_ENV").unwrap_or_else(|| "development".to_string());
        let is_development = environment.eq_ignore_ascii_case("development");

        let jwt_secret = match var("JWT_SECRET") {
            Some(secret) if !is_development => {
                if secret == EXAMPLE_JWT_SECRET || secret.chars().count() < MIN_JWT_SECRET_LEN {
                    bail!(
                        "JWT_SECRET must be a random string of at least {} characters outside development",
                        MIN_JWT_SECRET_LEN
                    );
                }
                secret
            }
            Some(secret) => secret,
            None if is_development => DEV_JWT_SECRET.to_string(),
            None => bail!("JWT_SECRET must be set when APP_ENV is '{}'", environment),
        };

        let allowed_origins = if is_development {
            None
        } else {
            let origins: Vec<String> = var("ALLOWED_ORIGINS")
                .unwrap_or_else(|| DEFAULT_ALLOWED_ORIGIN.to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if origins.is_empty() {
                bail!("ALLOWED_ORIGINS must contain at least one origin");
            }
            for origin in &origins {
                axum::http::HeaderValue::from_str(origin)
                    .with_context(|| format!("Invalid origin in ALLOWED_ORIGINS: {:?}", origin))?;
            }
            Some(origins)
        };

        let sentry_traces_sample_rate: f32 = parse_or(&var, "SENTRY_TRACES_SAMPLE_RATE", 0.1)?;
        if !(0.0..=1.0).contains(&sentry_traces_sample_rate) {
            bail!("SENTRY_TRACES_SAMPLE_RATE must be between 0.0 and 1.0");
        }

        let sync_interval_secs: u64 = parse_or(&var, "SYNC_INTERVAL_SECS", 3600)?;
        if sync_interval_secs == 0 {
            bail!("SYNC_INTERVAL_SECS must be greater than 0");
        }

        Ok(Self {
            database_url: var("DATABASE_URL").context("DATABASE_URL must be set")?,
            environment,
            is_development,
            jwt_secret,
            allowed_origins,
            sentry_dsn: var("SENTRY_DSN"),
            sentry_release: var("SENTRY_RELEASE"),
            sentry_traces_sample_rate,
            sync_interval_secs,
            sync_cooldown_secs: parse_or(&var, "SYNC_COOLDOWN_SECS", 60)?,
            port: parse_or(&var, "PORT", 3000)?,
        })
    }
}

/// Parse an optional variable, failing loudly on garbage instead of silently defaulting.
fn parse_or<T: FromStr>(
    var: &impl Fn(&str) -> Option<String>,
    key: &str,
    default: T,
) -> anyhow::Result<T> {
    match var(key) {
        None => Ok(default),
        Some(raw) => raw
            .parse()
            .map_err(|_| anyhow::anyhow!("{} has an invalid value: {:?}", key, raw)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const STRONG: &str = "0123456789abcdef0123456789abcdef";

    fn load(pairs: &[(&str, &str)]) -> anyhow::Result<Config> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Config::from_lookup(|k| map.get(k).cloned())
    }

    #[test]
    fn development_defaults() {
        let c = load(&[("DATABASE_URL", "postgres://x")]).unwrap();
        assert!(c.is_development);
        assert_eq!(c.jwt_secret, DEV_JWT_SECRET);
        assert_eq!(c.allowed_origins, None);
        assert_eq!(c.sync_interval_secs, 3600);
        assert_eq!(c.sync_cooldown_secs, 60);
        assert_eq!(c.port, 3000);
        assert_eq!(c.sentry_traces_sample_rate, 0.1);
    }

    #[test]
    fn any_non_development_env_requires_a_strong_secret() {
        for env in ["production", "staging", "test"] {
            assert!(
                load(&[("DATABASE_URL", "x"), ("APP_ENV", env)]).is_err(),
                "{env}"
            );
            assert!(load(&[
                ("DATABASE_URL", "x"),
                ("APP_ENV", env),
                ("JWT_SECRET", "short")
            ])
            .is_err());
            assert!(load(&[
                ("DATABASE_URL", "x"),
                ("APP_ENV", env),
                ("JWT_SECRET", EXAMPLE_JWT_SECRET)
            ])
            .is_err());
            let c = load(&[
                ("DATABASE_URL", "x"),
                ("APP_ENV", env),
                ("JWT_SECRET", STRONG),
            ])
            .unwrap();
            assert!(!c.is_development);
            assert_eq!(c.jwt_secret, STRONG);
        }
    }

    #[test]
    fn origins_outside_development() {
        let base = [
            ("DATABASE_URL", "x"),
            ("APP_ENV", "production"),
            ("JWT_SECRET", STRONG),
        ];
        let c = load(&base).unwrap();
        assert_eq!(
            c.allowed_origins,
            Some(vec![DEFAULT_ALLOWED_ORIGIN.to_string()])
        );

        let mut with = base.to_vec();
        with.push(("ALLOWED_ORIGINS", " https://a.com, https://b.com ,"));
        assert_eq!(
            load(&with).unwrap().allowed_origins,
            Some(vec![
                "https://a.com".to_string(),
                "https://b.com".to_string()
            ])
        );

        let mut bad = base.to_vec();
        bad.push(("ALLOWED_ORIGINS", "https://a.com\n"));
        // trailing whitespace is trimmed, so this is fine
        assert!(load(&bad).is_ok());
        let mut empty = base.to_vec();
        empty.push(("ALLOWED_ORIGINS", " , "));
        assert!(load(&empty).is_err());
    }

    #[test]
    fn invalid_numbers_fail_instead_of_defaulting() {
        assert!(load(&[("DATABASE_URL", "x"), ("PORT", "abc")]).is_err());
        assert!(load(&[("DATABASE_URL", "x"), ("SYNC_INTERVAL_SECS", "0")]).is_err());
        assert!(load(&[("DATABASE_URL", "x"), ("SENTRY_TRACES_SAMPLE_RATE", "2")]).is_err());
        assert_eq!(
            load(&[("DATABASE_URL", "x"), ("SYNC_INTERVAL_SECS", "120")])
                .unwrap()
                .sync_interval_secs,
            120
        );
    }

    #[test]
    fn database_url_required() {
        assert!(load(&[]).is_err());
    }
}
