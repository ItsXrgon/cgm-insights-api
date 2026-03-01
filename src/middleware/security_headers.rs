use axum::http::header;
use tower_http::set_header::SetResponseHeaderLayer;

/// Adds security headers to all responses
pub fn security_headers_layer() -> (
    SetResponseHeaderLayer<axum::http::HeaderValue>,
    SetResponseHeaderLayer<axum::http::HeaderValue>,
    SetResponseHeaderLayer<axum::http::HeaderValue>,
    SetResponseHeaderLayer<axum::http::HeaderValue>,
) {
    (
        SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            header::HeaderValue::from_static("nosniff"),
        ),
        SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            header::HeaderValue::from_static("DENY"),
        ),
        SetResponseHeaderLayer::overriding(
            header::X_XSS_PROTECTION,
            header::HeaderValue::from_static("1; mode=block"),
        ),
        SetResponseHeaderLayer::overriding(
            header::STRICT_TRANSPORT_SECURITY,
            header::HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        ),
    )
}
