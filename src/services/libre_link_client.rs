use crate::error::AppError;
use crate::models::NewGlucoseReading;
use libre_link_up_api_client::{LibreCgmData, LibreLinkUpClient, LibreLinkUpError};

/// LibreLink Up API client (delegates to libre_link_up_api_client crate).
pub struct LibreLinkClient {
    inner: LibreLinkUpClient,
}

impl LibreLinkClient {
    pub fn new(username: String, password: String, region: String) -> Result<Self, AppError> {
        let region_opt = if region.is_empty() {
            None
        } else {
            Some(region)
        };
        let inner = LibreLinkUpClient::simple(username, password, region_opt)
            .map_err(|e| AppError::ConfigError(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Fetch latest glucose readings from LibreLink Up via the crate.
    pub async fn fetch_latest_readings(
        &self,
        user_id: Option<i32>,
    ) -> Result<Vec<NewGlucoseReading>, AppError> {
        let data = self.inner.read().await.map_err(libre_error_to_app)?;
        let mut readings = Vec::new();
        readings.push(libre_cgm_to_reading(&data.current, None, user_id));
        for item in &data.history {
            readings.push(libre_cgm_to_reading(item, None, user_id));
        }
        Ok(readings)
    }

    /// No-op: the crate handles auth internally when calling read().
    pub async fn authenticate(&self) -> Result<(), AppError> {
        Ok(())
    }
}

fn libre_cgm_to_reading(
    d: &LibreCgmData,
    device_id: Option<String>,
    user_id: Option<i32>,
) -> NewGlucoseReading {
    NewGlucoseReading {
        user_id,
        value_mg_dl: d.value,
        timestamp: d.date,
        device_id,
        notes: None,
    }
}

fn libre_error_to_app(e: LibreLinkUpError) -> AppError {
    AppError::ConfigError(e.to_string())
}
