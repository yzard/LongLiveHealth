//! Display identities remain separate from editable, account-owned labels.
use crate::{
    database::{Database, queries},
    error::AppError,
};
use serde_json::{Value, json};
use sqlx::Row;
use std::collections::BTreeMap;

pub fn valid_key(key: &str) -> bool {
    key.len() <= 1024
        && !key.chars().any(char::is_control)
        && key.split_once(':').is_some_and(|(platform, id)| {
            [
                "apple_health",
                "health_connect",
                "manual",
                "file_import",
                "report-source",
                "report",
            ]
            .contains(&platform)
                && !id.trim().is_empty()
        })
}

pub fn report_key(name: Option<&str>, report_id: &str) -> String {
    name.map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| format!("report-source:{}", name.to_lowercase()))
        .unwrap_or_else(|| format!("report:{report_id}"))
}

pub async fn catalog(
    database: &Database,
    user: &str,
    health: &[Value],
) -> Result<Vec<Value>, AppError> {
    let mut catalog = BTreeMap::new();
    for source in health {
        let platform = source["platform"].as_str().ok_or(AppError::Internal)?;
        let id = source["source_id"].as_str().ok_or(AppError::Internal)?;
        if platform == "manual" && id == "longlivehealth.preferences" {
            continue;
        }
        let key = format!("{platform}:{id}");
        if catalog.contains_key(&key) {
            continue;
        }
        let name: Option<String> = sqlx::query_scalar(queries::SOURCE_DISPLAY_NAME)
            .bind(user)
            .bind(platform)
            .bind(id)
            .fetch_optional(&database.pool)
            .await?
            .flatten();
        catalog.insert(
            key.clone(),
            json!({"source_key":key,"name":name.filter(|n| !n.trim().is_empty()).unwrap_or(key)}),
        );
    }
    for row in sqlx::query(queries::REPORT_SOURCE_CATALOG)
        .bind(user)
        .fetch_all(&database.pool)
        .await?
    {
        let id: String = row.try_get("report_id")?;
        let context: Value = serde_json::from_str(&row.try_get::<String, _>("context_json")?)?;
        let name = context["source_name"]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let key = report_key(name, &id);
        let label = name
            .map(str::to_owned)
            .unwrap_or(row.try_get("original_name")?);
        catalog.insert(key.clone(), json!({"source_key":key,"name":label}));
    }
    Ok(catalog.into_values().collect())
}
