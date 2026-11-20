use crate::error::AppError;
use crate::models::{is_plausible_mg_dl, CgmCredential, CgmType, NewGlucoseReading, SyncStatus};
use crate::repositories::{cgm_repository, glucose_repository};
use crate::services::LibreLinkClient;
use crate::DbPool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{error, info, warn};

/// Region used for LibreLink when the credential has none.
const LIBRE_DEFAULT_REGION: &str = "eu";

#[derive(Debug, PartialEq, Eq)]
pub enum SyncOutcome {
    /// Rows inserted or updated
    Synced(usize),
    /// Another attempt for this credential started within the cooldown
    Skipped,
}

/// What a vendor client was built from; a change means the cached client is stale.
#[derive(Clone, PartialEq, Eq)]
struct LoginKey {
    username: String,
    password: String,
    region: String,
}

/// Fetches data from CGM platforms and stores it.
///
/// Keeps one vendor client per credential so its login token is reused across syncs
/// (logging in on every sync is slow and risks vendor lockouts).
pub struct SyncService {
    db: Arc<DbPool>,
    cooldown_secs: u64,
    libre_clients: Mutex<HashMap<i32, (LoginKey, Arc<LibreLinkClient>)>>,
}

impl SyncService {
    pub fn new(db: Arc<DbPool>, cooldown_secs: u64) -> Self {
        Self {
            db,
            cooldown_secs,
            libre_clients: Mutex::new(HashMap::new()),
        }
    }

    /// Sync one credential: claim it (respecting the cooldown, across processes),
    /// fetch and store readings, and record the outcome on the credential.
    pub async fn sync_credential(&self, cred: &CgmCredential) -> Result<SyncOutcome, AppError> {
        if !cgm_repository::claim_sync(&self.db, cred.id, self.cooldown_secs).await? {
            return Ok(SyncOutcome::Skipped);
        }

        info!(user_id = cred.user_id, cgm_type = %cred.cgm_type, "Starting CGM sync");
        let result = self.fetch_and_store(cred).await;

        let (status, message) = sync_status_of(&result);
        if let Err(e) = cgm_repository::record_sync_result(
            &self.db,
            cred.id,
            status.as_str(),
            message.as_deref(),
        )
        .await
        {
            error!(credential_id = cred.id, error = %e, "Failed to record sync result");
        }

        result.map(SyncOutcome::Synced)
    }

    /// Drop cached clients for credentials that are no longer active.
    pub fn retain_clients(&self, active_credential_ids: &[i32]) {
        self.libre_clients
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|id, _| active_credential_ids.contains(id));
    }

    async fn fetch_and_store(&self, cred: &CgmCredential) -> Result<usize, AppError> {
        let cgm_type: CgmType = cred.cgm_type.parse().map_err(|e| {
            AppError::ConfigError(format!(
                "Credential {} has invalid cgm_type: {}",
                cred.id, e
            ))
        })?;

        let fetched = self.fetch_readings(cgm_type, cred).await?;
        let fetched_count = fetched.len();
        let readings = prepare_readings(fetched, cred.user_id);

        if readings.len() < fetched_count {
            warn!(
                user_id = cred.user_id,
                dropped = fetched_count - readings.len(),
                "Dropped implausible readings from CGM"
            );
        }
        if readings.is_empty() {
            warn!(user_id = cred.user_id, "No readings returned from CGM");
            return Ok(0);
        }

        let stored = glucose_repository::insert_many(self.db.as_ref(), readings).await?;
        Ok(stored as usize)
    }

    /// The only place that knows which client serves which platform.
    /// To add a platform: add a `CgmType` variant, a client that returns
    /// `Vec<NewGlucoseReading>` in mg/dL, and an arm here.
    async fn fetch_readings(
        &self,
        cgm_type: CgmType,
        cred: &CgmCredential,
    ) -> Result<Vec<NewGlucoseReading>, AppError> {
        match cgm_type {
            CgmType::FreeStyle => self.libre_client(cred)?.fetch_latest_readings().await,
            CgmType::Dexcom => Err(AppError::ValidationError(
                "Dexcom sync is not yet implemented".to_string(),
            )),
        }
    }

    fn libre_client(&self, cred: &CgmCredential) -> Result<Arc<LibreLinkClient>, AppError> {
        let key = LoginKey {
            username: cred.username.clone(),
            password: cred.password.clone(),
            region: cred
                .region
                .clone()
                .unwrap_or_else(|| LIBRE_DEFAULT_REGION.to_string()),
        };

        let mut clients = self.libre_clients.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((cached_key, client)) = clients.get(&cred.id) {
            if *cached_key == key {
                return Ok(Arc::clone(client));
            }
        }

        let client = Arc::new(LibreLinkClient::new(
            key.username.clone(),
            key.password.clone(),
            Some(key.region.clone()),
        )?);
        clients.insert(cred.id, (key, Arc::clone(&client)));
        Ok(client)
    }
}

/// How a sync result is shown to the user. Internal failures get a generic message so
/// database or infrastructure details never reach the API.
pub(crate) fn sync_status_of(result: &Result<usize, AppError>) -> (SyncStatus, Option<String>) {
    match result {
        Ok(_) => (SyncStatus::Ok, None),
        Err(AppError::CgmCredentialsRejected(msg)) => {
            (SyncStatus::CredentialsRejected, Some(msg.clone()))
        }
        Err(AppError::CgmAccountLocked(msg)) => (SyncStatus::AccountLocked, Some(msg.clone())),
        Err(AppError::ApiError(msg)) | Err(AppError::ValidationError(msg)) => {
            (SyncStatus::Failed, Some(msg.clone()))
        }
        Err(_) => (
            SyncStatus::Failed,
            Some("Sync failed due to a server error; it will be retried".to_string()),
        ),
    }
}

/// Provider-agnostic cleanup applied to every platform's output:
/// attach the owning user and drop values that can't be real measurements.
pub(crate) fn prepare_readings(
    readings: Vec<NewGlucoseReading>,
    user_id: i32,
) -> Vec<NewGlucoseReading> {
    readings
        .into_iter()
        .filter(|r| is_plausible_mg_dl(r.value_mg_dl))
        .map(|r| NewGlucoseReading {
            user_id: Some(user_id),
            ..r
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn reading(value: f64, user_id: Option<i32>) -> NewGlucoseReading {
        NewGlucoseReading {
            user_id,
            value_mg_dl: value,
            timestamp: Utc::now(),
            is_high: false,
            is_low: false,
            trend: Some("flat".into()),
            device_id: None,
            notes: None,
        }
    }

    #[test]
    fn prepare_assigns_owner_and_overrides_provider_user() {
        let out = prepare_readings(vec![reading(100.0, None), reading(110.0, Some(99))], 5);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|r| r.user_id == Some(5)));
    }

    #[test]
    fn prepare_drops_implausible_values() {
        let out = prepare_readings(
            vec![
                reading(0.0, None),
                reading(-1.0, None),
                reading(f64::NAN, None),
                reading(5_000.0, None),
                reading(85.0, None),
            ],
            1,
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].value_mg_dl, 85.0);
    }

    #[test]
    fn status_mapping_hides_internal_errors() {
        assert_eq!(sync_status_of(&Ok(3)), (SyncStatus::Ok, None));
        assert_eq!(
            sync_status_of(&Err(AppError::CgmCredentialsRejected("bad pw".into()))),
            (SyncStatus::CredentialsRejected, Some("bad pw".into()))
        );
        assert_eq!(
            sync_status_of(&Err(AppError::CgmAccountLocked("wait".into()))).0,
            SyncStatus::AccountLocked
        );
        assert_eq!(
            sync_status_of(&Err(AppError::ApiError("vendor down".into()))),
            (SyncStatus::Failed, Some("vendor down".into()))
        );
        let (status, msg) = sync_status_of(&Err(AppError::DatabaseError("password=x".into())));
        assert_eq!(status, SyncStatus::Failed);
        assert!(!msg.unwrap().contains("password"));
    }

    fn credential(id: i32, cgm_type: &str, password: &str) -> CgmCredential {
        CgmCredential {
            id,
            user_id: 1,
            cgm_type: cgm_type.into(),
            username: "u@example.com".into(),
            password: password.into(),
            region: None,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_sync_at: None,
            last_sync_status: None,
            last_sync_error: None,
        }
    }

    fn service() -> SyncService {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://nobody@127.0.0.1:1/none")
            .unwrap();
        SyncService::new(Arc::new(pool.into()), 60)
    }

    #[tokio::test]
    async fn libre_client_is_reused_until_login_changes() {
        let svc = service();
        let a = svc
            .libre_client(&credential(1, "FreeStyle", "pw1"))
            .unwrap();
        let b = svc
            .libre_client(&credential(1, "FreeStyle", "pw1"))
            .unwrap();
        assert!(Arc::ptr_eq(&a, &b), "same login should reuse the client");

        let c = svc
            .libre_client(&credential(1, "FreeStyle", "pw2"))
            .unwrap();
        assert!(
            !Arc::ptr_eq(&a, &c),
            "changed password should build a new client"
        );

        let other = svc
            .libre_client(&credential(2, "FreeStyle", "pw2"))
            .unwrap();
        assert!(!Arc::ptr_eq(&c, &other), "clients are per credential");

        svc.retain_clients(&[2]);
        let d = svc
            .libre_client(&credential(1, "FreeStyle", "pw2"))
            .unwrap();
        assert!(!Arc::ptr_eq(&c, &d), "pruned clients are rebuilt");
    }

    #[tokio::test]
    async fn dexcom_is_rejected_without_network() {
        let err = service()
            .fetch_readings(CgmType::Dexcom, &credential(1, "Dexcom", "p"))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));
    }
}
