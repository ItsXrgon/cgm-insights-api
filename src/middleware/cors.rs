use axum::http::{header, Method};
use tower_http::cors::{Any, CorsLayer};

/// Configure CORS middleware
pub fn cors_layer() -> CorsLayer {
    let is_dev = std::env::var("ENVIRONMENT")
        .unwrap_or_else(|_| "development".to_string())
        .to_lowercase()
        == "development";

    let mut cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
            header::ORIGIN,
        ]);

    if is_dev {
        // Development: Allow any origin, no credentials
        cors = cors.allow_origin(Any);
    } else {
        // Production: Restrict origin, allow credentials
        cors = cors
            .allow_origin(
                std::env::var("ALLOWED_ORIGINS")
                    .unwrap_or_else(|_| "https://xrgon.com".to_string())
                    .parse::<axum::http::HeaderValue>()
                    .expect("Invalid CORS origin"),
            )
            .allow_credentials(true);
    }

    cors
}
