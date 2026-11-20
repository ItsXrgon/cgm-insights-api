use crate::dto::{CgmCredentialResponse, CreateCgmCredentialRequest, UpdateCgmCredentialRequest};
use crate::error::AppError;
use crate::models::{CgmCredential, CgmType, NewCgmCredential};
use crate::repositories::cgm_repository;
use crate::DbPool;

const CGM_CREDENTIAL_MAX_LEN: usize = 512;
const CGM_REGION_MAX_LEN: usize = 50;

fn to_response(cred: CgmCredential) -> CgmCredentialResponse {
    CgmCredentialResponse {
        id: cred.id,
        user_id: cred.user_id,
        cgm_type: cred.cgm_type,
        username: cred.username,
        region: cred.region,
        is_active: cred.is_active,
        created_at: cred.created_at.to_rfc3339(),
        updated_at: cred.updated_at.to_rfc3339(),
    }
}

fn not_found() -> AppError {
    AppError::NotFound("Credential not found".to_string())
}

/// Parse a user-supplied CGM type into its canonical stored name.
pub(crate) fn normalize_cgm_type(raw: &str) -> Result<String, AppError> {
    raw.parse::<CgmType>()
        .map(|t| t.as_str().to_string())
        .map_err(AppError::ValidationError)
}

/// Lowercase + trim; blank becomes `None` (provider default).
pub(crate) fn normalize_region(raw: Option<String>) -> Result<Option<String>, AppError> {
    let region = raw
        .map(|r| r.trim().to_lowercase())
        .filter(|r| !r.is_empty());
    if let Some(r) = &region {
        if r.chars().count() > CGM_REGION_MAX_LEN {
            return Err(AppError::ValidationError("CGM region is too long".to_string()));
        }
    }
    Ok(region)
}

fn validate_secret(field: &str, value: &str) -> Result<(), AppError> {
    if value.trim().is_empty() {
        return Err(AppError::ValidationError(format!("{} is required", field)));
    }
    if value.chars().count() > CGM_CREDENTIAL_MAX_LEN {
        return Err(AppError::ValidationError(format!("{} is too long", field)));
    }
    Ok(())
}

/// Validate and normalize a new credential. Shared by signup and `POST /api/cgm`.
pub(crate) fn build_new_credential(
    user_id: i32,
    cgm_type: &str,
    username: String,
    password: String,
    region: Option<String>,
    is_active: bool,
) -> Result<NewCgmCredential, AppError> {
    let cgm_type = normalize_cgm_type(cgm_type)?;
    validate_secret("CGM username", &username)?;
    validate_secret("CGM password", &password)?;
    Ok(NewCgmCredential {
        user_id,
        cgm_type,
        username: username.trim().to_string(),
        password,
        region: normalize_region(region)?,
        is_active,
    })
}

/// Load a credential, treating "owned by someone else" exactly like "missing".
async fn find_owned(
    pool: &DbPool,
    user_id: i32,
    credential_id: i32,
) -> Result<CgmCredential, AppError> {
    cgm_repository::find_by_id(pool, credential_id)
        .await?
        .filter(|c| c.user_id == user_id)
        .ok_or_else(not_found)
}

pub async fn list_credentials(
    pool: &DbPool,
    user_id: i32,
) -> Result<Vec<CgmCredentialResponse>, AppError> {
    let creds = cgm_repository::find_by_user_id(pool, user_id).await?;
    Ok(creds.into_iter().map(to_response).collect())
}

pub async fn add_credential(
    pool: &DbPool,
    user_id: i32,
    request: CreateCgmCredentialRequest,
) -> Result<CgmCredentialResponse, AppError> {
    let new_cred = build_new_credential(
        user_id,
        &request.cgm_type,
        request.cgm_username,
        request.cgm_password,
        request.cgm_region,
        false,
    )?;

    let mut cred = cgm_repository::insert(pool, new_cred).await?;
    if request.is_active {
        cred = cgm_repository::activate(pool, user_id, cred.id)
            .await?
            .ok_or_else(not_found)?;
    }
    Ok(to_response(cred))
}

pub async fn update_credential(
    pool: &DbPool,
    user_id: i32,
    credential_id: i32,
    request: UpdateCgmCredentialRequest,
) -> Result<CgmCredentialResponse, AppError> {
    find_owned(pool, user_id, credential_id).await?;

    let cgm_type = request
        .cgm_type
        .as_deref()
        .map(normalize_cgm_type)
        .transpose()?;
    if let Some(u) = &request.cgm_username {
        validate_secret("CGM username", u)?;
    }
    if let Some(p) = &request.cgm_password {
        validate_secret("CGM password", p)?;
    }
    let region = normalize_region(request.cgm_region)?;

    // Activation goes through `activate` so a user never ends up with two active rows.
    let is_active = match request.is_active {
        Some(true) => None,
        other => other,
    };

    let mut updated = cgm_repository::update(
        pool,
        credential_id,
        cgm_type,
        request.cgm_username.map(|u| u.trim().to_string()),
        request.cgm_password,
        region,
        is_active,
    )
    .await?;

    if request.is_active == Some(true) {
        updated = cgm_repository::activate(pool, user_id, credential_id)
            .await?
            .ok_or_else(not_found)?;
    }

    Ok(to_response(updated))
}

pub async fn set_active_credential(
    pool: &DbPool,
    user_id: i32,
    credential_id: i32,
) -> Result<CgmCredentialResponse, AppError> {
    let updated = cgm_repository::activate(pool, user_id, credential_id)
        .await?
        .ok_or_else(not_found)?;
    Ok(to_response(updated))
}

pub async fn delete_credential(
    pool: &DbPool,
    user_id: i32,
    credential_id: i32,
) -> Result<(), AppError> {
    if cgm_repository::delete(pool, user_id, credential_id).await? {
        Ok(())
    } else {
        Err(not_found())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(cgm_type: &str, user: &str, pass: &str, region: Option<&str>) -> Result<NewCgmCredential, AppError> {
        build_new_credential(
            7,
            cgm_type,
            user.to_string(),
            pass.to_string(),
            region.map(str::to_string),
            true,
        )
    }

    #[test]
    fn canonicalizes_type_and_region() {
        let c = build("freestyle", "  me@x.com ", "pw", Some(" EU ")).unwrap();
        assert_eq!(c.cgm_type, "FreeStyle");
        assert_eq!(c.username, "me@x.com");
        assert_eq!(c.region.as_deref(), Some("eu"));
        assert_eq!(c.user_id, 7);
        assert!(c.is_active);
    }

    #[test]
    fn blank_region_means_provider_default() {
        assert_eq!(build("FreeStyle", "u", "p", Some("   ")).unwrap().region, None);
        assert_eq!(build("FreeStyle", "u", "p", None).unwrap().region, None);
    }

    #[test]
    fn rejects_unknown_type() {
        assert!(matches!(
            build("unknown", "u", "p", None),
            Err(AppError::ValidationError(_))
        ));
    }

    #[test]
    fn rejects_empty_or_oversized_secrets() {
        assert!(build("FreeStyle", "", "p", None).is_err());
        assert!(build("FreeStyle", "u", "  ", None).is_err());
        let long = "x".repeat(CGM_CREDENTIAL_MAX_LEN + 1);
        assert!(build("FreeStyle", &long, "p", None).is_err());
        assert!(build("FreeStyle", "u", &long, None).is_err());
        let max = "x".repeat(CGM_CREDENTIAL_MAX_LEN);
        assert!(build("FreeStyle", &max, &max, None).is_ok());
    }

    #[test]
    fn password_is_not_trimmed() {
        assert_eq!(build("FreeStyle", "u", " p ", None).unwrap().password, " p ");
    }

    #[test]
    fn rejects_oversized_region() {
        let long = "r".repeat(CGM_REGION_MAX_LEN + 1);
        assert!(build("FreeStyle", "u", "p", Some(&long)).is_err());
    }
}
