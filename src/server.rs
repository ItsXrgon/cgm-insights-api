use crate::config::Config;
use crate::docs::openapi::{swagger_config, ApiDoc};
use crate::middleware::{cors_layer, jwt_auth, security_headers_layer};
use crate::services::SyncService;
use crate::DbPool;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::Router;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
};
use tower_http::compression::CompressionLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DbPool>,
    pub sync_service: Arc<SyncService>,
    pub config: Arc<Config>,
}

/// Build the router. Serve it with `into_make_service_with_connect_info::<SocketAddr>()`
/// so rate limiting can fall back to the peer IP when no proxy header is present.
pub fn create_app(state: AppState) -> Router {
    let sentry_enabled = state.config.sentry_dsn.is_some();

    let (h1, h2, h3, h4) = security_headers_layer();

    // Per client IP (X-Forwarded-For behind Fly's proxy, else the peer address), so one
    // busy client can't exhaust the limit for everyone.
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .key_extractor(SmartIpKeyExtractor)
            .per_second(2)
            .burst_size(5)
            .finish()
            .unwrap(),
    );

    let auth_governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .key_extractor(SmartIpKeyExtractor)
            .per_second(1)
            .burst_size(5)
            .finish()
            .unwrap(),
    );

    let middleware_stack = ServiceBuilder::new()
        .option_layer(if sentry_enabled {
            Some(sentry_tower::NewSentryLayer::<Request<Body>>::new_from_top())
        } else {
            None
        })
        .option_layer(if sentry_enabled {
            Some(sentry_tower::SentryHttpLayer::new().enable_transaction())
        } else {
            None
        })
        .layer(TraceLayer::new_for_http())
        .layer(h1)
        .layer(h2)
        .layer(h3)
        .layer(h4)
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(30),
        ))
        .layer(CompressionLayer::new())
        .layer(cors_layer(&state.config));

    let auth_routes = crate::handlers::auth::routes().layer(GovernorLayer {
        config: auth_governor_conf,
    });

    // Apply rate limiting only to protected API routes
    let api_routes = Router::new()
        .merge(crate::handlers::api_info::routes())
        .merge(crate::handlers::glucose::routes())
        .merge(crate::handlers::sync::routes())
        .merge(crate::handlers::cgm::routes())
        .merge(crate::handlers::insights::routes())
        .merge(crate::handlers::account::routes())
        .layer(from_fn_with_state(state.clone(), jwt_auth))
        .layer(GovernorLayer {
            config: governor_conf,
        });

    Router::new()
        .merge(crate::handlers::health::routes())
        .merge(
            SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", ApiDoc::openapi())
                .config(swagger_config()),
        )
        .nest("/api/auth", auth_routes)
        .nest("/api", api_routes)
        .with_state(state)
        .layer(middleware_stack)
}

/// Resolves on Ctrl-C or SIGTERM (what Fly and Docker send on deploy/stop).
pub async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %e, "Failed to listen for Ctrl-C");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to listen for SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("Shutdown signal received");
}
