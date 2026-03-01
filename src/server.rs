use crate::middleware::{cors_layer, jwt_auth, security_headers_layer};
use crate::services::SyncService;
use axum::http::StatusCode;
use axum::middleware::from_fn;
use axum::Router;
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_governor::{
    governor::GovernorConfigBuilder,
    key_extractor::GlobalKeyExtractor,
    GovernorLayer,
};
use tower_http::compression::CompressionLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub db: Pool<Postgres>,
    pub sync_service: Arc<SyncService>,
}

pub fn create_app(
    db: Pool<Postgres>,
    sync_service: Arc<SyncService>,
    sentry_enabled: bool,
) -> Router {
    let state = AppState { db, sync_service };

    let (h1, h2, h3, h4) = security_headers_layer();

    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .key_extractor(GlobalKeyExtractor)
            .per_second(2)
            .burst_size(5)
            .finish()
            .unwrap(),
    );

    let middleware_stack = ServiceBuilder::new()
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
        .layer(cors_layer())
        .layer(GovernorLayer {
            config: governor_conf,
        });

    let auth_routes = crate::handlers::auth::routes();

    let api_routes = Router::new()
        .merge(crate::handlers::api_info::routes())
        .merge(crate::handlers::glucose::routes())
        .merge(crate::handlers::sync::routes())
        .merge(crate::handlers::cgm::routes())
        .layer(from_fn(jwt_auth));

    Router::new()
        .merge(crate::handlers::health::routes())
        .nest("/api/auth", auth_routes)
        .nest("/api", api_routes)
        .with_state(state)
        .layer(middleware_stack)
}
