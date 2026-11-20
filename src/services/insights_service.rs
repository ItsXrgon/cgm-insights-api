use crate::dto::InsightsResponse;
use crate::error::AppError;
use crate::repositories::glucose_repository;
use crate::DbPool;
use chrono::{DateTime, Duration, Utc};

pub const DEFAULT_RANGE_DAYS: i64 = 14;
pub const MAX_RANGE_DAYS: i64 = 90;

/// Consensus CGM metrics (Battelino et al., Diabetes Care 2019) over a set of readings.
/// Time-in-range percentages are by reading count, which equals time when readings are
/// evenly spaced (true for a single sensor's stream).
#[derive(Debug, Clone, PartialEq)]
pub struct GlucoseStats {
    pub count: usize,
    pub mean_mg_dl: f64,
    /// Sample standard deviation
    pub sd_mg_dl: f64,
    /// Coefficient of variation, % (≤36% is considered stable)
    pub cv_percent: f64,
    /// Glucose management indicator, % (estimated A1c)
    pub gmi_percent: f64,
    pub min_mg_dl: f64,
    pub max_mg_dl: f64,
    /// < 54 mg/dL
    pub very_low_percent: f64,
    /// 54–69 mg/dL
    pub low_percent: f64,
    /// 70–180 mg/dL
    pub in_range_percent: f64,
    /// 181–250 mg/dL
    pub high_percent: f64,
    /// > 250 mg/dL
    pub very_high_percent: f64,
}

/// `None` when there are no readings.
pub fn compute_stats(values: &[f64]) -> Option<GlucoseStats> {
    if values.is_empty() {
        return None;
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let sd = if values.len() > 1 {
        (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt()
    } else {
        0.0
    };
    let pct =
        |pred: fn(f64) -> bool| values.iter().filter(|v| pred(**v)).count() as f64 / n * 100.0;

    Some(GlucoseStats {
        count: values.len(),
        mean_mg_dl: mean,
        sd_mg_dl: sd,
        cv_percent: sd / mean * 100.0,
        gmi_percent: 3.31 + 0.02392 * mean,
        min_mg_dl: values.iter().copied().fold(f64::INFINITY, f64::min),
        max_mg_dl: values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        very_low_percent: pct(|v| v < 54.0),
        low_percent: pct(|v| (54.0..70.0).contains(&v)),
        in_range_percent: pct(|v| (70.0..=180.0).contains(&v)),
        high_percent: pct(|v| v > 180.0 && v <= 250.0),
        very_high_percent: pct(|v| v > 250.0),
    })
}

/// Resolve the requested window: defaults to the last 14 days ending now, at most 90 days.
pub fn resolve_range(
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Result<(DateTime<Utc>, DateTime<Utc>), AppError> {
    let to = to.unwrap_or(now);
    let from = from.unwrap_or(to - Duration::days(DEFAULT_RANGE_DAYS));
    if from >= to {
        return Err(AppError::ValidationError(
            "'from' must be before 'to'".to_string(),
        ));
    }
    if to - from > Duration::days(MAX_RANGE_DAYS) {
        return Err(AppError::ValidationError(format!(
            "Range must be at most {} days",
            MAX_RANGE_DAYS
        )));
    }
    Ok((from, to))
}

pub async fn get_insights(
    pool: &DbPool,
    user_id: i32,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
) -> Result<InsightsResponse, AppError> {
    let (from, to) = resolve_range(from, to, Utc::now())?;
    let values = glucose_repository::values_in_range(pool, user_id, from, to).await?;
    Ok(InsightsResponse::new(from, to, compute_stats(&values)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn empty_has_no_stats() {
        assert_eq!(compute_stats(&[]), None);
    }

    #[test]
    fn single_reading() {
        let s = compute_stats(&[100.0]).unwrap();
        assert_eq!(s.count, 1);
        assert_eq!(s.sd_mg_dl, 0.0);
        assert_eq!(s.cv_percent, 0.0);
        assert_eq!(s.in_range_percent, 100.0);
    }

    #[test]
    fn mean_sd_cv_gmi() {
        let s = compute_stats(&[100.0, 120.0, 140.0, 160.0]).unwrap();
        assert!(close(s.mean_mg_dl, 130.0));
        // sample SD of 100,120,140,160 = sqrt(2000/3)
        assert!(close(s.sd_mg_dl, (2000.0f64 / 3.0).sqrt()));
        assert!(close(s.cv_percent, s.sd_mg_dl / 130.0 * 100.0));
        assert!(close(s.gmi_percent, 3.31 + 0.02392 * 130.0));
        assert_eq!((s.min_mg_dl, s.max_mg_dl), (100.0, 160.0));
    }

    #[test]
    fn range_boundaries_follow_consensus() {
        // one reading in each band, with the edges placed deliberately
        let s = compute_stats(&[53.9, 54.0, 69.9, 70.0, 180.0, 180.1, 250.0, 250.1]).unwrap();
        assert!(close(s.very_low_percent, 12.5)); // 53.9
        assert!(close(s.low_percent, 25.0)); // 54.0, 69.9
        assert!(close(s.in_range_percent, 25.0)); // 70.0, 180.0
        assert!(close(s.high_percent, 25.0)); // 180.1, 250.0
        assert!(close(s.very_high_percent, 12.5)); // 250.1
        let total = s.very_low_percent
            + s.low_percent
            + s.in_range_percent
            + s.high_percent
            + s.very_high_percent;
        assert!(close(total, 100.0));
    }

    #[test]
    fn range_defaults_and_limits() {
        let now = Utc.with_ymd_and_hms(2026, 11, 20, 12, 0, 0).unwrap();
        let (from, to) = resolve_range(None, None, now).unwrap();
        assert_eq!(to, now);
        assert_eq!(to - from, Duration::days(DEFAULT_RANGE_DAYS));

        let custom_to = now - Duration::days(30);
        let (from, to) = resolve_range(None, Some(custom_to), now).unwrap();
        assert_eq!(
            (to, to - from),
            (custom_to, Duration::days(DEFAULT_RANGE_DAYS))
        );

        assert!(resolve_range(Some(now), Some(now), now).is_err());
        assert!(resolve_range(Some(now - Duration::days(91)), Some(now), now).is_err());
        assert!(resolve_range(Some(now - Duration::days(90)), Some(now), now).is_ok());
    }
}
