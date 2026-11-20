use crate::dto::{AccountExport, ApiMessageResponse, DataResponse, DeleteAccountRequest};
use crate::error::AppError;
use crate::server::AppState;
use crate::services::{account_service, auth_service::Claims};
use axum::{
    extract::State,
    routing::{delete, get},
    Extension, Json, Router,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/account", delete(delete_account))
        .route("/account/export", get(export_account))
}

/// Download everything stored for this account (CGM passwords excluded)
#[utoipa::path(
    get,
    path = "/api/account/export",
    tag = "Account",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Account data", body = DataResponse<AccountExport>),
        (status = 401, description = "Unauthorized", body = crate::dto::ErrorResponse),
    ),
)]
pub async fn export_account(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<DataResponse<AccountExport>>, AppError> {
    let export = account_service::export(&state.db, claims.sub).await?;
    Ok(Json(DataResponse::new(export)))
}

/// Permanently delete the account with its credentials and readings
#[utoipa::path(
    delete,
    path = "/api/account",
    tag = "Account",
    security(("bearer_auth" = [])),
    request_body = DeleteAccountRequest,
    responses(
        (status = 200, description = "Account deleted", body = ApiMessageResponse),
        (status = 401, description = "Unauthorized or wrong password", body = crate::dto::ErrorResponse),
    ),
)]
pub async fn delete_account(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<DeleteAccountRequest>,
) -> Result<Json<ApiMessageResponse>, AppError> {
    account_service::delete(&state.db, claims.sub, &request.password).await?;
    Ok(Json(ApiMessageResponse::ok("Account deleted")))
}
