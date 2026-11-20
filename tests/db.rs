//! Tests against a real Postgres. Ignored by default; run with
//! `DATABASE_URL=postgres://... cargo test -- --include-ignored` (CI does).
//! Every test creates its own users with unique names, so tests can share one
//! database and run in parallel.

use axum::body::{to_bytes, Body};
use axum::extract::ConnectInfo;
use axum::http::{header, Method, Request, StatusCode};
use cgm_insights_api::error::AppError;
use cgm_insights_api::models::{NewCgmCredential, NewGlucoseReading, NewUser, User};
use cgm_insights_api::repositories::{cgm_repository, glucose_repository, user_repository};
use cgm_insights_api::server::AppState;
use cgm_insights_api::services::SyncService;
use cgm_insights_api::{create_app, db, Config, DbPool};
use chrono::{Duration, TimeZone, Utc};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tower::ServiceExt;

async fn pool() -> DbPool {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for DB tests");
    db::connect(&url).await.expect("connect + migrate")
}

fn unique(prefix: &str) -> String {
    static N: AtomicU32 = AtomicU32::new(0);
    format!(
        "{}_{}_{}",
        prefix,
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    )
}

fn new_cred(user_id: i32, active: bool) -> NewCgmCredential {
    NewCgmCredential {
        user_id,
        cgm_type: "FreeStyle".into(),
        username: "follower@example.com".into(),
        password: "pw".into(),
        region: Some("eu".into()),
        is_active: active,
    }
}

async fn user(pool: &DbPool) -> User {
    user_repository::insert_with_credential(
        pool,
        NewUser {
            username: unique("user"),
            password_hash: "x".into(),
        },
        new_cred(0, true),
    )
    .await
    .unwrap()
}

fn reading(user_id: i32, secs: i64, value: f64) -> NewGlucoseReading {
    NewGlucoseReading {
        user_id: Some(user_id),
        value_mg_dl: value,
        timestamp: Utc.timestamp_opt(secs, 0).unwrap(),
        is_high: false,
        is_low: false,
        trend: Some("flat".into()),
        device_id: None,
        notes: None,
    }
}

#[tokio::test]
#[ignore = "needs DATABASE_URL (Postgres)"]
async fn migrations_are_idempotent() {
    let p = pool().await;
    // connect() already migrated; running again must be a no-op.
    let url = std::env::var("DATABASE_URL").unwrap();
    db::connect(&url).await.unwrap();
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(n, db::MIGRATOR.iter().count() as i64);
}

#[tokio::test]
#[ignore = "needs DATABASE_URL (Postgres)"]
async fn signup_creates_user_and_credential_atomically() {
    let p = pool().await;
    let u = user(&p).await;
    let creds = cgm_repository::find_by_user_id(&p, u.id).await.unwrap();
    assert_eq!(creds.len(), 1);
    assert!(creds[0].is_active);

    // Duplicate username: conflict, and no orphan credential for anyone
    let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cgm_credentials")
        .fetch_one(&p)
        .await
        .unwrap();
    let err = user_repository::insert_with_credential(
        &p,
        NewUser {
            username: u.username.clone(),
            password_hash: "x".into(),
        },
        new_cred(0, true),
    )
    .await
    .unwrap_err();
    assert!(matches!(err, AppError::Conflict(_)));
    let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cgm_credentials")
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(before, after);
}

#[tokio::test]
#[ignore = "needs DATABASE_URL (Postgres)"]
async fn exactly_one_active_credential() {
    let p = pool().await;
    let u = user(&p).await;
    let first = cgm_repository::find_active_for_user(&p, u.id)
        .await
        .unwrap()
        .unwrap();
    let second = cgm_repository::insert(&p, new_cred(u.id, false))
        .await
        .unwrap();

    let activated = cgm_repository::activate(&p, u.id, second.id)
        .await
        .unwrap()
        .unwrap();
    assert!(activated.is_active);
    let active: Vec<i32> = cgm_repository::find_by_user_id(&p, u.id)
        .await
        .unwrap()
        .into_iter()
        .filter(|c| c.is_active)
        .map(|c| c.id)
        .collect();
    assert_eq!(active, vec![second.id]);

    // Back again, still exactly one
    cgm_repository::activate(&p, u.id, first.id)
        .await
        .unwrap()
        .unwrap();
    let active = cgm_repository::find_active_for_user(&p, u.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(active.id, first.id);

    // Someone else's credential: nothing changes
    let other = user(&p).await;
    assert!(cgm_repository::activate(&p, other.id, first.id)
        .await
        .unwrap()
        .is_none());
    assert!(cgm_repository::find_active_for_user(&p, u.id)
        .await
        .unwrap()
        .is_some());

    // The constraint itself rejects a second active row
    assert!(cgm_repository::insert(&p, new_cred(u.id, true))
        .await
        .is_err());
}

#[tokio::test]
#[ignore = "needs DATABASE_URL (Postgres)"]
async fn sync_claim_cooldown_and_status() {
    let p = pool().await;
    let u = user(&p).await;
    let cred = cgm_repository::find_active_for_user(&p, u.id)
        .await
        .unwrap()
        .unwrap();

    assert!(cgm_repository::claim_sync(&p, cred.id, 60).await.unwrap());
    assert!(
        !cgm_repository::claim_sync(&p, cred.id, 60).await.unwrap(),
        "within cooldown"
    );
    assert!(
        cgm_repository::claim_sync(&p, cred.id, 0).await.unwrap(),
        "no cooldown"
    );

    cgm_repository::record_sync_result(&p, cred.id, "credentials_rejected", Some("bad pw"))
        .await
        .unwrap();
    let c = cgm_repository::find_by_id(&p, cred.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(c.last_sync_status.as_deref(), Some("credentials_rejected"));
    assert_eq!(c.last_sync_error.as_deref(), Some("bad pw"));
    assert!(c.last_sync_at.is_some());

    // Toggling activity keeps the status...
    let c = cgm_repository::update(&p, cred.id, None, None, None, None, Some(true))
        .await
        .unwrap();
    assert_eq!(c.last_sync_status.as_deref(), Some("credentials_rejected"));

    // ...changing the password clears it and the cooldown
    let c = cgm_repository::update(&p, cred.id, None, None, Some("new".into()), None, None)
        .await
        .unwrap();
    assert_eq!(c.last_sync_status, None);
    assert_eq!(c.last_sync_error, None);
    assert_eq!(c.last_sync_at, None);
    assert!(cgm_repository::claim_sync(&p, cred.id, 60).await.unwrap());
}

#[tokio::test]
#[ignore = "needs DATABASE_URL (Postgres)"]
async fn bulk_upsert_dedupes_and_chunks() {
    let p = pool().await;
    let u = user(&p).await;

    // 2500 rows spans three insert chunks; one duplicate timestamp in the batch
    let mut batch: Vec<_> = (0..2500)
        .map(|i| reading(u.id, 1_000_000 + i * 60, 100.0))
        .collect();
    batch.push(reading(u.id, 1_000_000, 150.0));
    let affected = glucose_repository::insert_many(&p, batch).await.unwrap();
    assert_eq!(affected, 2500);

    // Re-sync overwrites instead of duplicating
    glucose_repository::insert_many(&p, vec![reading(u.id, 1_000_000, 90.0)])
        .await
        .unwrap();
    let all = glucose_repository::find_all_for_user(&p, u.id)
        .await
        .unwrap();
    assert_eq!(all.len(), 2500);
    assert_eq!(all[0].value_mg_dl, 90.0);
}

#[tokio::test]
#[ignore = "needs DATABASE_URL (Postgres)"]
async fn range_queries_and_cursor() {
    let p = pool().await;
    let u = user(&p).await;
    let other = user(&p).await;
    let base = Utc::now() - Duration::days(1);
    let t = |m: i64| base + Duration::minutes(m);
    let rows: Vec<_> = (0..10)
        .map(|m| NewGlucoseReading {
            timestamp: t(m * 5),
            ..reading(u.id, 0, 100.0 + m as f64)
        })
        .collect();
    glucose_repository::insert_many(&p, rows).await.unwrap();
    glucose_repository::insert_many(
        &p,
        vec![NewGlucoseReading {
            timestamp: t(0),
            ..reading(other.id, 0, 300.0)
        }],
    )
    .await
    .unwrap();

    // [from, to): includes from, excludes to; newest first; other users excluded
    let page = glucose_repository::find_range(&p, u.id, Some(t(5)), Some(t(25)), 100)
        .await
        .unwrap();
    let minutes: Vec<i64> = page
        .iter()
        .map(|r| (r.timestamp - base).num_minutes())
        .collect();
    assert_eq!(minutes, vec![20, 15, 10, 5]);

    // Paginate with the oldest timestamp as the next `to`
    let first = glucose_repository::find_range(&p, u.id, None, None, 4)
        .await
        .unwrap();
    let next = glucose_repository::find_range(&p, u.id, None, Some(first[3].timestamp), 4)
        .await
        .unwrap();
    assert!(next[0].timestamp < first[3].timestamp);
    assert_eq!(first.len() + next.len(), 8);

    let values = glucose_repository::values_in_range(&p, u.id, t(0), t(50))
        .await
        .unwrap();
    assert_eq!(values.len(), 10);
}

#[tokio::test]
#[ignore = "needs DATABASE_URL (Postgres)"]
async fn deleting_a_user_cascades() {
    let p = pool().await;
    let u = user(&p).await;
    glucose_repository::insert_many(&p, vec![reading(u.id, 5_000_000, 100.0)])
        .await
        .unwrap();
    assert!(user_repository::delete(&p, u.id).await.unwrap());
    assert!(cgm_repository::find_by_user_id(&p, u.id)
        .await
        .unwrap()
        .is_empty());
    assert!(glucose_repository::find_all_for_user(&p, u.id)
        .await
        .unwrap()
        .is_empty());
    assert!(!user_repository::delete(&p, u.id).await.unwrap());
}

/// Full API flow on a real database: signup, login, readings, insights, export, delete.
#[tokio::test]
#[ignore = "needs DATABASE_URL (Postgres)"]
async fn end_to_end_api_flow() {
    let p = Arc::new(pool().await);
    let vars = HashMap::from([("DATABASE_URL", "unused")]);
    let config = Arc::new(Config::from_lookup(|k| vars.get(k).map(|v| v.to_string())).unwrap());
    let app = create_app(AppState {
        sync_service: Arc::new(SyncService::new(Arc::clone(&p), 60)),
        db: p,
        config,
    });

    // 7 rate-limited calls: 2 auth (burst 5) and 5 API (burst 5), so no 429s
    let call = |method: Method, uri: &str, token: Option<&str>, body: Option<serde_json::Value>| {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(t) = token {
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {}", t));
        }
        let mut req = match body {
            Some(b) => builder
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(b.to_string()))
                .unwrap(),
            None => builder.body(Body::empty()).unwrap(),
        };
        req.extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([10, 0, 0, 1], 1))));
        let app = app.clone();
        async move {
            let res = app.oneshot(req).await.unwrap();
            let status = res.status();
            let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
            (
                status,
                serde_json::from_slice::<serde_json::Value>(&bytes).unwrap_or_default(),
            )
        }
    };

    let (status, _) = call(Method::GET, "/health", None, None).await;
    assert_eq!(status, StatusCode::OK);

    let username = unique("e2e");
    let password = "long-enough-password";
    let (status, body) = call(
        Method::POST,
        "/api/auth/signup",
        None,
        Some(serde_json::json!({
            "username": username, "password": password,
            "cgmUsername": "f@example.com", "cgmPassword": "pw", "cgmType": "freestyle"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");

    let (status, body) = call(
        Method::POST,
        "/api/auth/login",
        None,
        Some(serde_json::json!({ "username": username, "password": password })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let token = body["data"]["token"].as_str().unwrap().to_string();

    let (status, _) = call(
        Method::POST,
        "/api/glucose",
        Some(&token),
        Some(serde_json::json!({ "valueMgDl": 120 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, body) = call(Method::GET, "/api/insights", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["count"], 1);
    assert_eq!(body["data"]["timeInRanges"]["inRange"], 100.0);

    let (status, body) = call(Method::GET, "/api/account/export", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["glucoseReadings"].as_array().unwrap().len(), 1);
    assert_eq!(body["data"]["cgmCredentials"][0]["cgmType"], "FreeStyle");
    assert!(
        !body.to_string().contains("\"pw\""),
        "export must not leak CGM passwords"
    );

    let (status, _) = call(
        Method::DELETE,
        "/api/account",
        Some(&token),
        Some(serde_json::json!({ "password": "wrong-password" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _) = call(
        Method::DELETE,
        "/api/account",
        Some(&token),
        Some(serde_json::json!({ "password": password })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}
