//! OpenAPI specification and Swagger UI configuration.

use utoipa::OpenApi;
use utoipa_swagger_ui::Config;

use crate::dto::{
    ApiInfoResponse, ApiMessageResponse, AuthDataResponse, AuthResponse, CgmCredentialResponse,
    CreateCgmCredentialRequest, CreateGlucoseReadingRequest, ErrorResponse, GlucoseReadingResponse,
    HealthResponse, LoginRequest, SignupRequest, UpdateCgmCredentialRequest, UserResponse,
};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "CGM Insights API",
        version = env!("CARGO_PKG_VERSION"),
        description = "A Rust server for CGM integrations (LibreLinkUp, Dexcom, etc.)",
        license(name = "MIT"),
    ),
    paths(
        crate::handlers::health::health_check,
        crate::handlers::api_info::api_info,
        crate::handlers::auth::signup,
        crate::handlers::auth::login,
        crate::handlers::glucose::create_reading,
        crate::handlers::glucose::get_readings,
        crate::handlers::glucose::get_reading,
        crate::handlers::glucose::delete_reading,
        crate::handlers::sync::trigger_sync,
        crate::handlers::cgm::list_credentials,
        crate::handlers::cgm::add_credential,
        crate::handlers::cgm::update_credential,
        crate::handlers::cgm::set_active,
        crate::handlers::cgm::delete_credential,
    ),
    components(schemas(
        HealthResponse,
        ApiInfoResponse,
        SignupRequest,
        LoginRequest,
        AuthResponse,
        AuthDataResponse,
        UserResponse,
        CreateGlucoseReadingRequest,
        crate::handlers::glucose::ListQuery,
        GlucoseReadingResponse,
        CgmCredentialResponse,
        CreateCgmCredentialRequest,
        UpdateCgmCredentialRequest,
        ApiMessageResponse,
        ErrorResponse,
    )),
    tags(
        (name = "Health", description = "Health and API info"),
        (name = "Auth", description = "Authentication (signup, login)"),
        (name = "Glucose", description = "Glucose readings"),
        (name = "Sync", description = "Manual sync trigger"),
        (name = "CGM", description = "CGM credentials management"),
    ),
    modifiers(&SecurityAddon),
)]
pub struct ApiDoc;

/// Adds Bearer auth to protected paths
struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::Http::builder()
                        .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some("JWT token from login"))
                        .build(),
                ),
            );
        }
    }
}

/// Swagger UI config so the UI finds openapi.json when served at a nested path
pub fn swagger_config() -> Config<'static> {
    Config::from("/api-docs/openapi.json")
}
