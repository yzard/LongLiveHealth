//! Complete period queries with streaming chart reduction and paginated history.
use crate::{
    database::{Database, queries},
    error::AppError,
};
use futures_util::TryStreamExt;
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::Row;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub record_type: String,
    pub timezone: String,
    pub period: String,
    pub start_at: i64,
    pub end_at: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryRequest {
    pub record_type: String,
    pub start_at: i64,
    pub end_at: i64,
    pub offset: u32,
    pub expected_revision: Option<i64>,
}
pub fn unit(kind: &str) -> Result<&'static str, AppError> {
    match kind {
        "heart_rate" | "respiratory_rate" => Ok("count/min"),
        "blood_glucose" => Ok("mg/dL"),
        "vo2_max" => Ok("ml/kg*min"),
        "body_mass" => Ok("kg"),
        "hrv_rmssd" | crate::recovery_hrv::KIND => Ok("ms"),
        "height" => Ok("m"),
        "body_temperature" => Ok("degC"),
        "body_fat" | "oxygen_saturation" => Ok("%"),
        _ => {
            crate::apple_health::unit(kind).ok_or(AppError::Invalid("Unsupported measurement type"))
        }
    }
}
fn validate(start: i64, end: i64) -> Result<(), AppError> {
    if end <= start
        || i128::from(end) - i128::from(start) > 367 * 86400
        || chrono::DateTime::from_timestamp(start, 0).is_none()
        || chrono::DateTime::from_timestamp(end, 0).is_none()
    {
        return Err(AppError::Invalid(
            "Select a measurement period of at most 367 days",
        ));
    }
    Ok(())
}
impl Database {
    pub async fn measurement_chart(&self, user: &str, r: Request) -> Result<Value, AppError> {
        validate(r.start_at, r.end_at)?;
        if super::combined_chart::supports(&r.record_type) {
            return self.combined_chart(user, &r).await;
        }
        let unit = unit(&r.record_type)?;
        let tz = r
            .timezone
            .parse()
            .map_err(|_| AppError::Invalid("Unknown IANA timezone"))?;
        let mut ranges = crate::chart::PeriodRanges::new(&r.period, tz)
            .ok_or(AppError::Invalid("Unknown chart period"))?;
        let mut daily = crate::chart::PeriodRanges::new("M", tz).ok_or(AppError::Internal)?;
        let revision = self.data_revision(user).await?;
        let bounds: (Option<i64>, Option<i64>) = sqlx::query_as(queries::MEASUREMENT_BOUNDS)
            .bind(user)
            .bind(&r.record_type)
            .fetch_one(&self.pool)
            .await?;
        let mut stream = sqlx::query(queries::MEASUREMENT_SAMPLES)
            .bind(user)
            .bind(&r.record_type)
            .bind(r.start_at)
            .bind(r.end_at)
            .fetch(&self.pool);
        let mut read = 0usize;
        let mut valid = 0usize;
        let mut sources = std::collections::BTreeSet::new();
        while let Some(row) = stream.try_next().await? {
            read += 1;
            let platform: String = row.try_get("platform")?;
            let raw_unit: Option<String> = row.try_get("unit")?;
            let raw_value: Option<f64> = row.try_get("value")?;
            if let Some(value) =
                super::series::convert(&platform, &r.record_type, raw_unit.as_deref(), raw_value)
            {
                valid += 1;
                let source = format!("{}:{}", platform, row.try_get::<String, _>("source_id")?);
                sources.insert(source.clone());
                daily.add(source.clone(), unit.into(), row.try_get("at")?, value);
                ranges.add(source, unit.into(), row.try_get("at")?, value);
            }
        }
        drop(stream);
        if revision != self.data_revision(user).await? {
            return Err(AppError::Conflict("Archive changed; refresh the chart"));
        }
        let mut chart_ranges = ranges.finish();
        chart_ranges["reference_days"] = daily.finish()["M"].take();
        Ok(
            json!({"chart_ranges":chart_ranges,"unit":unit,"first_at":bounds.0,"last_at":bounds.1,"sources":sources,"samples_read":read,"valid_samples":valid,"excluded_samples":read-valid,"complete":true,"data_revision":revision}),
        )
    }
    pub async fn measurement_history(
        &self,
        user: &str,
        r: HistoryRequest,
    ) -> Result<Value, AppError> {
        validate(r.start_at, r.end_at)?;
        let unit = unit(&r.record_type)?;
        let revision = self.data_revision(user).await?;
        if r.expected_revision.is_some_and(|v| v != revision) {
            return Err(AppError::Conflict("Archive changed; reload history"));
        }
        let query = queries::measurement_history_query();
        let rows = sqlx::query(&query)
            .bind(user)
            .bind(&r.record_type)
            .bind(r.start_at)
            .bind(r.end_at)
            .bind(i64::from(r.offset))
            .fetch_all(&self.pool)
            .await?;
        let more = rows.len() > 500;
        let mut points = Vec::new();
        for row in rows.into_iter().take(500) {
            let platform: String = row.try_get("platform")?;
            let raw_unit: Option<String> = row.try_get("unit")?;
            let raw_value: Option<f64> = row.try_get("value")?;
            points.push(json!({"at":row.try_get::<f64,_>("at")?,"source":format!("{}:{}",platform,row.try_get::<String,_>("source_id")?),"record_id":row.try_get::<String,_>("record_id")?,"value":super::series::convert(&platform,&r.record_type,raw_unit.as_deref(),raw_value),"raw_value":raw_value,"raw_unit":raw_unit,"unit":unit}));
        }
        if revision != self.data_revision(user).await? {
            return Err(AppError::Conflict("Archive changed; reload history"));
        }
        Ok(
            json!({"points":points,"next_offset":if more {Some(r.offset.checked_add(500).ok_or(AppError::Invalid("History offset is too large"))?)}else{None},"data_revision":revision}),
        )
    }
}
