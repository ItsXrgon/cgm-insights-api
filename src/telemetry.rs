//! Logging and Sentry setup.

use crate::Config;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Routes whose transactions are never sampled (noise: health checks, docs).
const EXCLUDED_TRANSACTION_ROUTES: &[&str] = &["/health", "/swagger-ui", "/api-docs"];

/// Initialize tracing, plus Sentry when `SENTRY_DSN` is set. Keep the returned guard
/// alive for the whole process so buffered events are flushed on exit.
pub fn init(config: &Config) -> Option<sentry::ClientInitGuard> {
    let guard = config.sentry_dsn.as_deref().map(|dsn| {
        let traces_sample_rate = config.sentry_traces_sample_rate;
        sentry::init((
            dsn,
            sentry::ClientOptions {
                release: Some(release_name(config).into()),
                environment: Some(config.environment.clone().into()),
                // Health data: don't attach IPs, cookies or request bodies by default.
                send_default_pii: false,
                attach_stacktrace: true,
                traces_sample_rate,
                traces_sampler: Some(Arc::new(move |tx| {
                    if is_excluded_transaction_route(tx.name(), EXCLUDED_TRANSACTION_ROUTES) {
                        0.0
                    } else {
                        traces_sample_rate
                    }
                })),
                auto_session_tracking: true,
                session_mode: sentry::SessionMode::Request,
                ..Default::default()
            },
        ))
    });

    let registry = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env());
    if guard.is_some() {
        registry.with(sentry::integrations::tracing::layer()).init();
        tracing::info!(environment = %config.environment, "Sentry enabled");
    } else {
        registry.init();
        tracing::info!("Sentry disabled (SENTRY_DSN not set)");
    }

    guard
}

fn release_name(config: &Config) -> String {
    config.sentry_release.clone().unwrap_or_else(|| {
        let rev = env!("GIT_REV_SHORT");
        let version = if rev == "unknown" {
            env!("CARGO_PKG_VERSION").to_string()
        } else {
            format!("{}-{}", env!("CARGO_PKG_VERSION"), rev)
        };
        format!("{}@{}", env!("CARGO_PKG_NAME"), version)
    })
}

/// Transaction names look like `GET /health?x=1`; match on the path only.
fn is_excluded_transaction_route(tx_name: &str, excluded_routes: &[&str]) -> bool {
    let path_with_query = tx_name.split_whitespace().last().unwrap_or(tx_name);
    let path = path_with_query.split('?').next().unwrap_or(path_with_query);

    excluded_routes.iter().any(|route| {
        path == *route || path.starts_with(&format!("{}/", route.trim_end_matches('/')))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excludes_routes_and_their_subpaths_only() {
        let ex = EXCLUDED_TRANSACTION_ROUTES;
        assert!(is_excluded_transaction_route("GET /health", ex));
        assert!(is_excluded_transaction_route("GET /health?probe=1", ex));
        assert!(is_excluded_transaction_route(
            "GET /swagger-ui/index.html",
            ex
        ));
        assert!(is_excluded_transaction_route("/api-docs/openapi.json", ex));
        assert!(!is_excluded_transaction_route("GET /healthz", ex));
        assert!(!is_excluded_transaction_route("GET /api/glucose", ex));
    }
}
