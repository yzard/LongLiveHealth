//! Operator-only capture of Browse response payloads for cross-platform UI verification.
use crate::{database::Database, error::AppError};
use serde_json::{Value, json};
use std::path::Path;

pub async fn run(
    db: &Database,
    user: &str,
    timezone: String,
    output: &Path,
) -> Result<(), AppError> {
    tokio::fs::create_dir_all(output).await?;
    let catalog = crate::browse_catalog::get(db, user).await?;
    let preferences = db.wellness_preferences(user).await?;
    let sources = db.wellness_sources(user).await?;
    let profile = serde_json::from_value(preferences["preferences"]["reference_profile"].clone())
        .map_err(|_| AppError::Internal)?;
    let mut cases = Vec::new();
    for (index, entry) in catalog["entries"]
        .as_array()
        .ok_or(AppError::Internal)?
        .iter()
        .enumerate()
    {
        let kind = entry["kind"].as_str().ok_or(AppError::Internal)?;
        let presentation = entry["presentation"].as_str().unwrap_or("");
        let response = async {
            let mut result = match presentation {
                "lab" => {
                    crate::metric_history::history(
                        db,
                        user,
                        crate::metric_history::HistoryRequest {
                            metric_id: kind.into(),
                        },
                    )
                    .await?
                }
                "daily" | "sleep" => {
                    db.daily_history(
                        user,
                        crate::wellness::history::HistoryRequest {
                            record_type: kind.into(),
                            timezone: timezone.clone(),
                            dates: None,
                        },
                    )
                    .await?
                }
                "measurement" => {
                    let now = crate::authentication::now()?;
                    let mut request = crate::wellness::measurement_chart::Request {
                        record_type: kind.into(),
                        timezone: timezone.clone(),
                        period: "Y".into(),
                        start_at: now - 366 * 86400,
                        end_at: now + 1,
                    };
                    let initial = db.measurement_chart(user, request.clone()).await?;
                    if let Some(last) = initial["last_at"].as_i64() {
                        request.end_at = last + 1;
                        request.start_at = last - 366 * 86400;
                    }
                    db.measurement_chart(user, request).await?
                }
                _ => {
                    let mut records = db
                        .specialty_records(
                            user,
                            crate::wellness::records::ListRequest {
                                kind: kind.into(),
                                after: None,
                            },
                        )
                        .await?;
                    if entry["definition"]["family"] == "characteristic" {
                        while !records["next_cursor"].is_null() {
                            let after = serde_json::from_value(records["next_cursor"].clone())
                                .map_err(|_| AppError::Internal)?;
                            let page = db
                                .specialty_records(
                                    user,
                                    crate::wellness::records::ListRequest {
                                        kind: kind.into(),
                                        after: Some(after),
                                    },
                                )
                                .await?;
                            records["records"]
                                .as_array_mut()
                                .ok_or(AppError::Internal)?
                                .extend(
                                    page["records"]
                                        .as_array()
                                        .ok_or(AppError::Internal)?
                                        .iter()
                                        .cloned(),
                                );
                            records["next_cursor"] = page["next_cursor"].clone();
                        }
                        let latest = records["records"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .max_by_key(|r| (r["at"].as_i64(), r["version"].as_i64()))
                            .cloned();
                        if let Some(latest) = latest {
                            records["latest_detail"] = db
                                .specialty_record(
                                    user,
                                    crate::wellness::records::GetRequest {
                                        platform: latest["platform"]
                                            .as_str()
                                            .ok_or(AppError::Internal)?
                                            .into(),
                                        source_id: latest["source_id"]
                                            .as_str()
                                            .ok_or(AppError::Internal)?
                                            .into(),
                                        record_id: latest["record_id"]
                                            .as_str()
                                            .ok_or(AppError::Internal)?
                                            .into(),
                                        sample_offset: 0,
                                    },
                                )
                                .await?;
                        }
                    }
                    if kind == "electrocardiogram"
                        && let Some(first) = records["records"]
                            .as_array()
                            .and_then(|r| r.first())
                            .cloned()
                    {
                        records["latest_detail"] = db
                            .specialty_record(
                                user,
                                crate::wellness::records::GetRequest {
                                    platform: first["platform"]
                                        .as_str()
                                        .ok_or(AppError::Internal)?
                                        .into(),
                                    source_id: first["source_id"]
                                        .as_str()
                                        .ok_or(AppError::Internal)?
                                        .into(),
                                    record_id: first["record_id"]
                                        .as_str()
                                        .ok_or(AppError::Internal)?
                                        .into(),
                                    sample_offset: 0,
                                },
                            )
                            .await?;
                    }
                    return Ok::<Value, AppError>(records);
                }
            };
            if let Some(components) = result["components"].as_array_mut() {
                for component in components {
                    let id = component["id"]
                        .as_str()
                        .ok_or(AppError::Internal)?
                        .to_owned();
                    crate::reference_guides::annotate(component, &id, &profile);
                }
            } else {
                if kind == "body_mass" {
                    crate::reference_guides::add_height_context(db, user, &profile, &mut result)
                        .await?;
                }
                crate::reference_guides::annotate(&mut result, kind, &profile);
            }
            Ok(result)
        }
        .await;
        let value = match response {
            Ok(value) => value,
            Err(error) => json!({"capture_error":error.to_string()}),
        };
        let file = format!("{index}.json");
        tokio::fs::write(output.join(&file), serde_json::to_vec(&value)?).await?;
        cases.push(json!({"entry":entry,"file":file}));
        println!(
            "{}",
            json!({"captured":index+1,"metric":kind,"error":value["capture_error"]})
        );
    }
    tokio::fs::write(
        output.join("manifest.json"),
        serde_json::to_vec(
            &json!({"catalog":catalog,"preferences":preferences,"sources":sources,"cases":cases}),
        )?,
    )
    .await?;
    Ok(())
}

/// Re-run the production annotator over private captured payloads without querying the archive.
pub async fn reassess(output: &Path) -> Result<(), AppError> {
    let manifest: Value =
        serde_json::from_slice(&tokio::fs::read(output.join("manifest.json")).await?)?;
    let profile =
        serde_json::from_value(manifest["preferences"]["preferences"]["reference_profile"].clone())
            .map_err(|_| AppError::Internal)?;
    for case in manifest["cases"].as_array().ok_or(AppError::Internal)? {
        let file = case["file"].as_str().ok_or(AppError::Internal)?;
        if Path::new(file).components().count() != 1 {
            return Err(AppError::Invalid("Invalid capture filename"));
        }
        let mut value: Value = serde_json::from_slice(&tokio::fs::read(output.join(file)).await?)?;
        if let Some(components) = value["components"].as_array_mut() {
            for component in components {
                let kind = component["id"]
                    .as_str()
                    .ok_or(AppError::Internal)?
                    .to_owned();
                crate::reference_guides::annotate(component, &kind, &profile);
            }
        } else if value["chart_ranges"].is_object() {
            let kind = case["entry"]["kind"].as_str().ok_or(AppError::Internal)?;
            crate::reference_guides::annotate(&mut value, kind, &profile);
        }
        tokio::fs::write(output.join(file), serde_json::to_vec(&value)?).await?;
    }
    Ok(())
}
