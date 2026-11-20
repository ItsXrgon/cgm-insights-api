use crate::dto::{
    ApiMessageResponse, CgmCredentialResponse, CreateCgmCredentialRequest, DataResponse,
    UpdateCgmCredentialRequest,
};
use crate::error::AppError;
use crate::server::AppState;
use crate::services::{auth_service::Claims, cgm_service};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Extension, Json, Router,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/cgm", get(list_credentials).post(add_credential))
        .route(
            "/cgm/{id}",
            axum::routing::patch(update_credential).delete(delete_credential),
        )
        .route("/cgm/{id}/active", post(set_active))
}

/// List all CGM credentials for the current user, with their latest sync status
#[utoipa::path(
    get,
    path = "/api/cgm",
    tag = "CGM",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of credentials", body = DataResponse<Vec<CgmCredentialResponse>>),
        (status = 401, description = "Unauthorized", body = crate::dto::ErrorResponse),
    ),
)]
pub async fn list_credentials(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<DataResponse<Vec<CgmCredentialResponse>>>, AppError> {
    let creds = cgm_service::list_credentials(&state.db, claims.sub).await?;
    Ok(Json(DataResponse::new(creds)))
}

/// Add a new CGM credential
#[utoipa::path(
    post,
    path = "/api/cgm",
    tag = "CGM",
    security(("bearer_auth" = [])),
    request_body = CreateCgmCredentialRequest,
    responses(
        (status = 201, description = "Credential created", body = DataResponse<CgmCredentialResponse>),
        (status = 400, description = "Invalid input", body = crate::dto::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::dto::ErrorResponse),
    ),
)]
pub async fn add_credential(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<CreateCgmCredentialRequest>,
) -> Result<(StatusCode, Json<DataResponse<CgmCredentialResponse>>), AppError> {
    let cred = cgm_service::add_credential(&state.db, claims.sub, request).await?;
    Ok((StatusCode::CREATED, Json(DataResponse::new(cred))))
}

/// Update an existing CGM credential
#[utoipa::path(
    patch,
    path = "/api/cgm/{id}",
    tag = "CGM",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Credential ID")),
    request_body = UpdateCgmCredentialRequest,
    responses(
        (status = 200, description = "Credential updated", body = DataResponse<CgmCredentialResponse>),
        (status = 400, description = "Invalid input", body = crate::dto::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::dto::ErrorResponse),
        (status = 404, description = "Not found", body = crate::dto::ErrorResponse),
    ),
)]
pub async fn update_credential(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i32>,
    Json(request): Json<UpdateCgmCredentialRequest>,
) -> Result<Json<DataResponse<CgmCredentialResponse>>, AppError> {
    let cred = cgm_service::update_credential(&state.db, claims.sub, id, request).await?;
    Ok(Json(DataResponse::new(cred)))
}

/// Set a credential as the active one for syncing
#[utoipa::path(
    post,
    path = "/api/cgm/{id}/active",
    tag = "CGM",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Credential ID")),
    responses(
        (status = 200, description = "Credential set as active", body = DataResponse<CgmCredentialResponse>),
        (status = 401, description = "Unauthorized", body = crate::dto::ErrorResponse),
        (status = 404, description = "Not found", body = crate::dto::ErrorResponse),
    ),
)]
pub async fn set_active(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i32>,
) -> Result<Json<DataResponse<CgmCredentialResponse>>, AppError> {
    let cred = cgm_service::set_active_credential(&state.db, claims.sub, id).await?;
    Ok(Json(DataResponse::new(cred)))
}

/// Delete a CGM credential
#[utoipa::path(
    delete,
    path = "/api/cgm/{id}",
    tag = "CGM",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Credential ID")),
    responses(
        (status = 200, description = "Credential deleted", body = ApiMessageResponse),
        (status = 401, description = "Unauthorized", body = crate::dto::ErrorResponse),
        (status = 404, description = "Not found", body = crate::dto::ErrorResponse),
    ),
)]
pub async fn delete_credential(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i32>,
) -> Result<Json<ApiMessageResponse>, AppError> {
    cgm_service::delete_credential(&state.db, claims.sub, id).await?;
    Ok(Json(ApiMessageResponse::ok(
        "Credential deleted successfully",
    )))
}
