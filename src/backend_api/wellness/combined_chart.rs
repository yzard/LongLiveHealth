//! Paired/component measurements resolved in SQL, within one account and source.
use super::measurement_chart::Request;
use crate::{
    database::{Database, queries},
    error::AppError,
};
use futures_util::TryStreamExt;
use serde_json::{Value, json};
use sqlx::Row;
use std::collections::{BTreeMap, BTreeSet};

pub fn supports(kind: &str) -> bool {
    crate::apple_health::definition(kind).is_some_and(|d| d.family == "correlation")
        || super::component_projection::supports(kind)
}

impl Database {
    pub async fn combined_chart(&self, user: &str, r: &Request) -> Result<Value, AppError> {
        let tz = r
            .timezone
            .parse()
            .map_err(|_| AppError::Invalid("Unknown IANA timezone"))?;
        let revision = self.data_revision(user).await?;
        let bounds: (Option<i64>, Option<i64>) = sqlx::query_as(queries::COMBINED_BOUNDS)
            .bind(user)
            .bind(&r.record_type)
            .fetch_one(&self.pool)
            .await?;
        let mut stream = sqlx::query(queries::COMBINED_SAMPLES)
            .bind(user)
            .bind(&r.record_type)
            .bind(r.start_at)
            .bind(r.end_at)
            .fetch(&self.pool);
        let mut series =
            BTreeMap::<String, (crate::chart::PeriodRanges, crate::chart::PeriodRanges)>::new();
        let mut definitions = BTreeMap::new();
        let mut sources = BTreeSet::new();
        let mut seen = BTreeSet::new();
        let mut records = BTreeSet::new();
        let mut excluded = 0usize;
        let mut latest: Option<(i64, String, Value)> = None;
        while let Some(row) = stream.try_next().await? {
            let at: i64 = row.try_get("at")?;
            let record: String = row.try_get("record_id")?;
            let platform: String = row.try_get("platform")?;
            let source = format!("{}:{}", platform, row.try_get::<String, _>("source_id")?);
            records.insert((source.clone(), record.clone()));
            let kind: Option<String> = row.try_get("component")?;
            let Some(kind) = kind else {
                excluded += 1;
                continue;
            };

            let raw_unit: Option<String> = row.try_get("unit")?;
            let raw_value: Option<f64> = row.try_get("value")?;
            let canonical = super::measurement_chart::unit(&kind).ok();
            let unit = canonical.or(raw_unit.as_deref()).unwrap_or("");
            let value = if canonical.is_some() {
                super::series::convert(&platform, &kind, raw_unit.as_deref(), raw_value)
            } else {
                raw_value.filter(|v| v.is_finite())
            };
            let Some(value) = value else {
                excluded += 1;
                continue;
            };
            // A correlation may repeat a child ID. Never pair by approximate time or across sources.
            let component_record: String = row.try_get("component_record")?;
            if !seen.insert((source.clone(), record.clone(), component_record)) {
                continue;
            }
            let title: Option<String> = row.try_get("title")?;
            let title = title
                .as_deref()
                .or_else(|| crate::apple_health::definition(&kind).map(|d| d.title.as_str()))
                .unwrap_or(&kind);
            definitions.insert(kind.clone(), json!({"id":kind,"title":title,"unit":unit}));
            if !series.contains_key(&kind) {
                series.insert(
                    kind.clone(),
                    (
                        crate::chart::PeriodRanges::new(&r.period, tz)
                            .ok_or(AppError::Invalid("Unknown chart period"))?,
                        crate::chart::PeriodRanges::new("M", tz).ok_or(AppError::Internal)?,
                    ),
                );
            }
            let (period, daily) = series.get_mut(&kind).ok_or(AppError::Internal)?;
            period.add(source.clone(), unit.into(), at as f64, value);
            daily.add(source.clone(), unit.into(), at as f64, value);
            let identity = format!("{source}:{record}");
            if latest
                .as_ref()
                .is_none_or(|(previous, key, _)| (at, &identity) > (*previous, key))
            {
                latest = Some((
                    at,
                    identity.clone(),
                    json!({"at":at,"source":source,"record_id":record,"values":{}}),
                ));
            }
            if let Some((_, key, reading)) = &mut latest
                && *key == identity
            {
                reading["values"][&kind] = json!(value);
            }
            sources.insert(source);
        }
        drop(stream);
        if revision != self.data_revision(user).await? {
            return Err(AppError::Conflict("Archive changed; refresh the chart"));
        }
        let mut components = Vec::new();
        for (kind, (period, daily)) in series {
            let mut ranges = period.finish();
            ranges["reference_days"] = daily.finish()["M"].take();
            if kind == "event_count" {
                for rows in ranges
                    .as_object_mut()
                    .into_iter()
                    .flat_map(|p| p.values_mut())
                {
                    for row in rows.as_array_mut().into_iter().flatten() {
                        for key in ["min", "max", "mean", "latest_value"] {
                            row[key] = row["count"].clone();
                        }
                    }
                }
            }
            let mut component = definitions.remove(&kind).ok_or(AppError::Internal)?;
            component["chart_ranges"] = ranges;
            components.push(component);
        }
        Ok(
            json!({"components":components,"sources":sources,"first_at":bounds.0,"last_at":bounds.1,
            "samples_read":records.len(),"valid_samples":seen.len(),"excluded_samples":excluded,
            "latest_reading":latest.map(|(_,_,r)|r),"complete":true,"data_revision":revision}),
        )
    }
}
