//! Complete source-separated daily history, independent of Overview's baseline window.
use crate::{
    database::{Database, queries},
    error::AppError,
    health::AggregateRequest,
};
use chrono::{Days, TimeZone};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryRequest {
    pub record_type: String,
    pub timezone: String,
    pub dates: Option<[String; 2]>,
}
impl Database {
    pub async fn daily_history(
        &self,
        user: &str,
        request: HistoryRequest,
    ) -> Result<Value, AppError> {
        crate::health::daily_unit(&request.record_type)?;
        let tz: chrono_tz::Tz = request
            .timezone
            .parse()
            .map_err(|_| AppError::Invalid("Unknown IANA timezone"))?;
        let revision = self.data_revision(user).await?;
        let (first, last): (Option<i64>, Option<i64>) =
            sqlx::query_as(queries::DAILY_HISTORY_BOUNDS)
                .bind(user)
                .bind(&request.record_type)
                .fetch_one(&self.pool)
                .await?;
        let mut days = Vec::new();
        let mut end_date = chrono::Utc::now().with_timezone(&tz).date_naive();
        if let (Some(first), Some(last)) = (first, last) {
            let date = |at| {
                tz.timestamp_opt(at, 0)
                    .single()
                    .map(|v| v.date_naive())
                    .ok_or(AppError::Invalid("Invalid history date"))
            };
            let mut cursor = date(first)?;
            end_date = date(last)?;
            if let Some([start, end]) = &request.dates {
                cursor = chrono::NaiveDate::parse_from_str(start, "%Y-%m-%d")
                    .map_err(|_| AppError::Invalid("Invalid history start date"))?;
                end_date = chrono::NaiveDate::parse_from_str(end, "%Y-%m-%d")
                    .map_err(|_| AppError::Invalid("Invalid history end date"))?;
                if end_date < cursor || (end_date - cursor).num_days() > 366 {
                    return Err(AppError::Invalid("Select at most 367 history days"));
                }
            }
            if (end_date - cursor).num_days() > 20000 {
                return Err(AppError::TooLarge);
            }
            while cursor <= end_date {
                let end = cursor
                    .checked_add_days(Days::new(365))
                    .ok_or(AppError::Internal)?
                    .min(end_date);
                let result = self
                    .aggregate_health(
                        user,
                        AggregateRequest {
                            record_type: request.record_type.clone(),
                            start_date: cursor.to_string(),
                            end_date: end.to_string(),
                            timezone: request.timezone.clone(),
                        },
                    )
                    .await?;
                days.extend(
                    result["days"]
                        .as_array()
                        .ok_or(AppError::Internal)?
                        .iter()
                        .cloned(),
                );
                if days.len() > 100000 {
                    return Err(AppError::TooLarge);
                }
                cursor = end.succ_opt().ok_or(AppError::Internal)?;
            }
        }
        if revision != self.data_revision(user).await? {
            return Err(AppError::Conflict("Archive changed; refresh history"));
        }
        let mut result = super::select_metric(
            &request.record_type,
            &end_date.to_string(),
            &[],
            &json!({"days":days,"algorithm_version":crate::health::ALGORITHM_VERSION}),
        )?;
        result["data_revision"] = json!(revision);
        result["first_record_at"] = json!(first);
        result["last_record_at"] = json!(last);
        Ok(result)
    }
}
