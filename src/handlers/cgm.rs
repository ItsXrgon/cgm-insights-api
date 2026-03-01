use crate::dto::{CreateCgmCredentialRequest, UpdateCgmCredentialRequest};
use crate::error::AppError;
use crate::server::AppState;
use crate::services::{auth_service::Claims, cgm_service};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
    Extension, Json, Router,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/cgm", get(list_credentials))
        .route("/cgm", post(add_credential))
        .route("/cgm/{id}", patch(update_credential))
        .route("/cgm/{id}/active", post(set_active))
        .route("/cgm/{id}", delete(delete_credential))
}

async fn list_credentials(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, AppError> {
    let creds = cgm_service::list_credentials(&state.db, claims.sub).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": creds
    })))
}

async fn add_credential(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<CreateCgmCredentialRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let cred = cgm_service::add_credential(&state.db, claims.sub, request).await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "success": true,
            "data": cred
        })),
    ))
}

async fn update_credential(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i32>,
    Json(request): Json<UpdateCgmCredentialRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let cred = cgm_service::update_credential(&state.db, claims.sub, id, request).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": cred
    })))
}

async fn set_active(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    let cred = cgm_service::set_active_credential(&state.db, claims.sub, id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": cred
    })))
}

async fn delete_credential(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i32>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let deleted = cgm_service::delete_credential(&state.db, claims.sub, id).await?;

    if deleted {
        Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": "Credential deleted successfully"
            })),
        ))
    } else {
        Ok((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "success": false,
                "message": "Credential not found"
            })),
        ))
    }
}
