//! HTTP-level tests for routing, auth middleware, CORS and validation paths that are
//! rejected before touching the database. The pool is lazy and points nowhere, so a
//! test that accidentally reaches Postgres fails instead of silently passing.

use axum::body::{to_bytes, Body};
use axum::extract::ConnectInfo;
use axum::http::{header, HeaderMap, Method, Request, StatusCode};
use axum::Router;
use cgm_insights_api::models::User;
use cgm_insights_api::services::{generate_token, SyncService};
use cgm_insights_api::{create_app, DbPool};
use std::net::SocketAddr;
use std::sync::Arc;
use tower::ServiceExt;

fn app() -> Router {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(200))
        .connect_lazy("postgres://nobody:nothing@127.0.0.1:1/none")
        .unwrap();
    let db: Arc<DbPool> = Arc::new(pool.into());
    let sync = Arc::new(SyncService::new(Arc::clone(&db)));
    create_app(db, sync, false)
}

/// Sends the request the way `axum::serve(.. into_make_service_with_connect_info ..)`
/// delivers it: with the peer address attached and no proxy headers.
async fn send(mut req: Request<Body>) -> (StatusCode, HeaderMap, serde_json::Value) {
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 40000))));
    let res = app().oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, headers, json)
}

fn req(method: Method, uri: &str) -> axum::http::request::Builder {
    Request::builder().method(method).uri(uri)
}

fn json_post(uri: &str, body: serde_json::Value) -> Request<Body> {
    req(Method::POST, uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn bearer(user_id: i32) -> String {
    let now = chrono::Utc::now();
    let token = generate_token(&User {
        id: user_id,
        username: "tester".into(),
        password_hash: String::new(),
        created_at: now,
        updated_at: now,
    })
    .unwrap();
    format!("Bearer {}", token)
}

#[tokio::test]
async fn health_is_public_and_sets_security_headers() {
    let (status, headers, body) = send(req(Method::GET, "/health").body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "OK");
    assert_eq!(headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
    assert_eq!(headers[header::X_FRAME_OPTIONS], "DENY");
}

#[tokio::test]
async fn protected_routes_require_a_valid_token() {
    for (method, uri) in [
        (Method::GET, "/api/glucose"),
        (Method::POST, "/api/sync"),
        (Method::GET, "/api/cgm"),
        (Method::DELETE, "/api/cgm/1"),
    ] {
        let (status, _, _) = send(req(method.clone(), uri).body(Body::empty()).unwrap()).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri} without token");

        let (status, _, _) = send(
            req(method.clone(), uri)
                .header(header::AUTHORIZATION, "Bearer not-a-jwt")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri} with bad token");
    }
}

#[tokio::test]
async fn valid_token_passes_auth_middleware() {
    let (status, _, body) = send(
        req(Method::GET, "/api")
            .header(header::AUTHORIZATION, bearer(1))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "CGM Insights Server");
}

#[tokio::test]
async fn cors_preflight_allows_patch() {
    let (status, headers, _) = send(
        req(Method::OPTIONS, "/api/cgm/1")
            .header(header::ORIGIN, "http://localhost:5173")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "PATCH")
            .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "authorization,content-type")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert!(status.is_success());
    let allowed = headers[header::ACCESS_CONTROL_ALLOW_METHODS].to_str().unwrap();
    assert!(allowed.contains("PATCH"), "allowed methods: {allowed}");
}

#[tokio::test]
async fn auth_routes_work_without_proxy_headers() {
    // Regression: the IP-keyed rate limiter answered 500 when there was no proxy header
    // and the server didn't provide ConnectInfo (i.e. any non-Fly deployment).
    let (status, _, body) = send(json_post(
        "/api/auth/login",
        serde_json::json!({ "username": "ab", "password": "short" }),
    ))
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("Username"));
}

#[tokio::test]
async fn signup_rejects_unknown_cgm_type_before_creating_user() {
    let (status, _, body) = send(json_post(
        "/api/auth/signup",
        serde_json::json!({
            "username": "alice",
            "password": "long-enough-password",
            "cgmUsername": "me@example.com",
            "cgmPassword": "pw",
            "cgmType": "Medtronic"
        }),
    ))
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("Unsupported CGM type"));
}

#[tokio::test]
async fn manual_reading_rejects_implausible_value() {
    let (status, _, _) = send(
        req(Method::POST, "/api/glucose")
            .header(header::AUTHORIZATION, bearer(1))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"valueMgDl": 0}"#))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
