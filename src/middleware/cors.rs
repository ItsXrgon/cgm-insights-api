use crate::config::Config;
use axum::http::{header, HeaderValue, Method};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

/// CORS: any origin in development, `Config::allowed_origins` everywhere else.
pub fn cors_layer(config: &Config) -> CorsLayer {
    let cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
            header::ORIGIN,
        ]);

    match &config.allowed_origins {
        None => cors.allow_origin(Any),
        Some(origins) => {
            // Validated in Config::from_lookup
            let origins: Vec<HeaderValue> = origins
                .iter()
                .filter_map(|o| HeaderValue::from_str(o).ok())
                .collect();
            cors.allow_origin(AllowOrigin::list(origins))
                .allow_credentials(true)
        }
    }
}
