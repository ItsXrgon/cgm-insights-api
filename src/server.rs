use crate::docs::openapi::{swagger_config, ApiDoc};
use crate::middleware::{cors_layer, jwt_auth, security_headers_layer};
use crate::services::SyncService;
use crate::DbPool;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::from_fn;
use axum::Router;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_governor::{
    governor::GovernorConfigBuilder,
    key_extractor::{GlobalKeyExtractor, SmartIpKeyExtractor},
    GovernorLayer,
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
}

pub fn create_app(db: Arc<DbPool>, sync_service: Arc<SyncService>, sentry_enabled: bool) -> Router {
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

    // Stricter per-IP rate limit for auth (login/signup) to prevent brute force and spam
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
        .layer(cors_layer());

    let auth_routes = crate::handlers::auth::routes().layer(GovernorLayer {
        config: auth_governor_conf,
    });

    // Apply rate limiting only to protected API routes (glucose, sync, cgm)
    // Exclude health, Swagger UI, api-docs, and auth - they load many assets or need to stay unthrottled
    let api_routes = Router::new()
        .merge(crate::handlers::api_info::routes())
        .merge(crate::handlers::glucose::routes())
        .merge(crate::handlers::sync::routes())
        .merge(crate::handlers::cgm::routes())
        .layer(from_fn(jwt_auth))
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
