//! Operator diagnostic: exercise Browse's real query paths and print counts, not health values.
use crate::{database::Database, error::AppError};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn bars(value: &Value) -> usize {
    value["chart_ranges"]["D"].as_array().map_or(0, Vec::len)
}

pub async fn run(
    db: &Database,
    user: &str,
    date: String,
    timezone: String,
) -> Result<(), AppError> {
    let catalog = db.wellness_sources(user).await?;
    let total = |kind: &str| -> i64 {
        catalog["sources"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|s| s["record_type"] == kind)
            .filter_map(|s| s["record_count"].as_i64())
            .sum()
    };
    let daily = db
        .compute_wellness_day(
            user,
            crate::wellness::DayRequest {
                date,
                timezone: timezone.clone(),
                source_priority: BTreeMap::new(),
            },
        )
        .await?;
    for m in daily["metrics"].as_array().into_iter().flatten() {
        println!(
            "{}",
            json!({"section":"daily","metric":m["record_type"],"stored":total(m["record_type"].as_str().ok_or(AppError::Internal)?),"bars":bars(m),"state":m["current"]["state"]})
        );
    }
    for kind in [
        "sleep",
        "steps",
        "workout",
        "resting_heart_rate",
        "hrv_sdnn",
        "active_energy",
        "basal_energy",
    ] {
        let result = db
            .daily_history(
                user,
                crate::wellness::history::HistoryRequest {
                    dates: None,
                    record_type: kind.into(),
                    timezone: timezone.clone(),
                },
            )
            .await;
        match result {
            Ok(v) => println!(
                "{}",
                json!({"section":"full_daily_history","metric":kind,"stored":total(kind),"bars":bars(&v),"first_record_at":v["first_record_at"],"first_plotted_at":v["chart_ranges"]["D"].as_array().into_iter().flatten().filter_map(|r|r["start_at"].as_i64()).min(),"last_plotted_at":v["chart_ranges"]["D"].as_array().into_iter().flatten().filter_map(|r|r["end_at"].as_i64()).max()})
            ),
            Err(e) => println!(
                "{}",
                json!({"section":"full_daily_history","metric":kind,"error":e.to_string()})
            ),
        }
    }
    let end = crate::authentication::now()?;
    for definition in crate::apple_health::definitions()
        .iter()
        .filter(|d| d.family == "quantity" && d.browse && total(&d.kind) > 0)
    {
        let kind = definition.kind.as_str();
        let mut earliest = None;
        for period in ["H", "D", "W", "M", "6M", "Y"] {
            let width = match period {
                "H" => 3600,
                "D" => 86400,
                "W" => 7 * 86400,
                "M" => 31 * 86400,
                "6M" => 184 * 86400,
                _ => 366 * 86400,
            };
            for historical in [false, true] {
                let start = if historical {
                    earliest.unwrap_or(end - width)
                } else {
                    end - width
                };
                let stop = if historical { start + width } else { end };
                let result = db
                    .measurement_chart(
                        user,
                        crate::wellness::measurement_chart::Request {
                            record_type: kind.into(),
                            timezone: timezone.clone(),
                            period: period.into(),
                            start_at: start,
                            end_at: stop,
                        },
                    )
                    .await;
                match result {
                    Ok(v) => {
                        earliest = v["first_at"].as_i64();
                        println!(
                            "{}",
                            json!({"section":"measurement_period","metric":kind,"period":period,"historical":historical,"stored":total(kind),"start_at":start,"end_at":stop,"first_at":v["first_at"],"last_at":v["last_at"],"samples":v["samples_read"],"excluded":v["excluded_samples"],"complete":v["complete"],"bars":v["chart_ranges"][period].as_array().map_or(0,Vec::len)})
                        );
                    }
                    Err(e) => println!(
                        "{}",
                        json!({"section":"measurement_period","metric":kind,"period":period,"historical":historical,"error":e.to_string()})
                    ),
                }
            }
        }
    }
    let sleep = db
        .sleep_sessions(
            user,
            crate::wellness::sleep::SleepRequest {
                start_at: end - 30 * 86400,
                end_at: end,
                timezone,
            },
        )
        .await?;
    println!(
        "{}",
        json!({"section":"sleep_sessions","sources":sleep["sources"].as_array().map_or(0,Vec::len)})
    );
    let labs = crate::metric_history::browse(db, user).await?;
    for metric in labs["metrics"].as_array().into_iter().flatten() {
        let id = metric["metric_id"].as_str().ok_or(AppError::Internal)?;
        let result = crate::metric_history::history(
            db,
            user,
            crate::metric_history::HistoryRequest {
                metric_id: id.into(),
            },
        )
        .await;
        match result {
            Ok(v) => println!(
                "{}",
                json!({"section":"labs","metric":id,"name":metric["name"],"records":v["records"].as_array().map_or(0,Vec::len),"points":v["points"].as_array().map_or(0,Vec::len),"bars":bars(&v),"excluded":v["excluded_count"]})
            ),
            Err(e) => println!(
                "{}",
                json!({"section":"labs","metric":id,"error":e.to_string()})
            ),
        }
    }
    Ok(())
}
