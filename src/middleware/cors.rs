use axum::http::{header, HeaderValue, Method};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

/// Configure CORS middleware.
/// In production (APP_ENV=production), uses ALLOWED_ORIGINS (comma-separated).
/// In development, allows any origin.
pub fn cors_layer() -> CorsLayer {
    let is_dev = std::env::var("APP_ENV")
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
        cors = cors.allow_origin(Any);
    } else {
        let origins_str = std::env::var("ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "https://xrgon.com".to_string());
        let origins: Vec<HeaderValue> = origins_str
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.parse::<HeaderValue>().unwrap_or_else(|_| {
                    panic!("Invalid CORS origin in ALLOWED_ORIGINS: {:?}", s)
                })
            })
            .collect();
        if origins.is_empty() {
            panic!("ALLOWED_ORIGINS must contain at least one origin in production");
        }
        cors = cors
            .allow_origin(AllowOrigin::list(origins))
            .allow_credentials(true);
    }

    cors
}
