use crate::error::AppError;
use crate::models::NewGlucoseReading;
use libre_link_up_api_client::{LibreCgmData, LibreLinkUpClient, LibreLinkUpError, TrendType};

/// LibreLink Up API client (delegates to libre_link_up_api_client crate).
pub struct LibreLinkClient {
    inner: LibreLinkUpClient,
}

impl LibreLinkClient {
    /// `region: None` lets the crate use its default region.
    pub fn new(username: String, password: String, region: Option<String>) -> Result<Self, AppError> {
        let inner =
            LibreLinkUpClient::simple(username, password, region).map_err(libre_error_to_app)?;
        Ok(Self { inner })
    }

    /// Fetch the current reading plus recent history. `user_id` is left unset;
    /// `SyncService` assigns it.
    pub async fn fetch_latest_readings(&self) -> Result<Vec<NewGlucoseReading>, AppError> {
        let data = self.inner.read().await.map_err(libre_error_to_app)?;
        Ok(std::iter::once(&data.current)
            .chain(&data.history)
            .map(libre_cgm_to_reading)
            .collect())
    }
}

fn libre_cgm_to_reading(d: &LibreCgmData) -> NewGlucoseReading {
    NewGlucoseReading {
        user_id: None,
        value_mg_dl: d.value,
        timestamp: d.timestamp,
        is_high: d.is_high,
        is_low: d.is_low,
        trend: Some(trend_type_to_string(&d.trend).to_string()),
        device_id: None,
        notes: None,
    }
}

/// Map the crate's TrendType to a stable camelCase string for storage and API.
/// These strings are part of the public API; other providers must map onto the same set.
fn trend_type_to_string(t: &TrendType) -> &'static str {
    match t {
        TrendType::SingleDown => "singleDown",
        TrendType::FortyFiveDown => "fortyFiveDown",
        TrendType::Flat => "flat",
        TrendType::FortyFiveUp => "fortyFiveUp",
        TrendType::SingleUp => "singleUp",
        TrendType::NotComputable => "notComputable",
    }
}

/// Classify LibreLink failures: account problems the user must fix vs. temporary
/// lockouts vs. upstream/transport failures (ours or theirs, worth alerting on).
fn libre_error_to_app(e: LibreLinkUpError) -> AppError {
    let msg = e.to_string();
    match e {
        LibreLinkUpError::AccountLocked(_) => AppError::CgmAccountLocked(msg),
        LibreLinkUpError::BadCredentials
        | LibreLinkUpError::AuthFailed(_)
        | LibreLinkUpError::AdditionalActionRequired(_)
        | LibreLinkUpError::RegionNotFound(..)
        | LibreLinkUpError::NoConnections
        | LibreLinkUpError::ConnectionNotFound(_) => AppError::CgmCredentialsRejected(msg),
        LibreLinkUpError::Http(_)
        | LibreLinkUpError::SerializationError(_)
        | LibreLinkUpError::InvalidResponse(_)
        | LibreLinkUpError::ConnectionFunctionFailed => AppError::ApiError(msg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn maps_cgm_data_to_reading() {
        let ts = Utc.with_ymd_and_hms(2026, 1, 2, 3, 4, 5).unwrap();
        let r = libre_cgm_to_reading(&LibreCgmData {
            value: 123.0,
            is_high: true,
            is_low: false,
            trend: TrendType::FortyFiveUp,
            timestamp: ts,
        });
        assert_eq!(r.value_mg_dl, 123.0);
        assert_eq!(r.timestamp, ts);
        assert!(r.is_high && !r.is_low);
        assert_eq!(r.trend.as_deref(), Some("fortyFiveUp"));
        assert_eq!(r.user_id, None);
    }

    #[test]
    fn trend_strings_are_stable() {
        let all = [
            (TrendType::SingleDown, "singleDown"),
            (TrendType::FortyFiveDown, "fortyFiveDown"),
            (TrendType::Flat, "flat"),
            (TrendType::FortyFiveUp, "fortyFiveUp"),
            (TrendType::SingleUp, "singleUp"),
            (TrendType::NotComputable, "notComputable"),
        ];
        for (t, s) in all {
            assert_eq!(trend_type_to_string(&t), s);
        }
    }

    #[test]
    fn classifies_errors() {
        assert!(matches!(
            libre_error_to_app(LibreLinkUpError::AccountLocked(300)),
            AppError::CgmAccountLocked(m) if m.contains("300")
        ));
        for e in [
            LibreLinkUpError::BadCredentials,
            LibreLinkUpError::NoConnections,
            LibreLinkUpError::AuthFailed("x".into()),
            LibreLinkUpError::AdditionalActionRequired("tou".into()),
            LibreLinkUpError::RegionNotFound("zz".into(), "eu, us".into()),
        ] {
            assert!(matches!(libre_error_to_app(e), AppError::CgmCredentialsRejected(_)));
        }
        assert!(matches!(
            libre_error_to_app(LibreLinkUpError::InvalidResponse("bad".into())),
            AppError::ApiError(_)
        ));
    }
}
