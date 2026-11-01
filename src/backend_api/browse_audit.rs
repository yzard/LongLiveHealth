//! Operator diagnostic: exercise Browse's real query paths and print counts, not health values.
use crate::{database::Database, error::AppError};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn bars(value: &Value) -> usize {
    value["chart_ranges"]["D"].as_array().map_or(0, Vec::len)
}

async fn audit_reference(
    db: &Database,
    user: &str,
    value: &mut Value,
    metric: &str,
    profile: &crate::reference_guides::Profile,
) {
    if metric == "body_mass"
        && let Err(error) =
            crate::reference_guides::add_height_context(db, user, profile, value).await
    {
        println!(
            "{}",
            json!({"section":"reference_audit","metric":metric,"error":error.to_string()})
        );
        return;
    }
    crate::reference_guides::annotate(value, metric, profile);
    let mut counts = BTreeMap::<String, usize>::new();
    let mut reasons = std::collections::BTreeSet::new();
    for row in value["chart_ranges"]
        .as_object()
        .into_iter()
        .flat_map(|p| p.values())
        .flat_map(|v| v.as_array().into_iter().flatten())
    {
        let a = &row["reference_assessment"];
        *counts
            .entry(a["status"].as_str().unwrap_or("missing_assessment").into())
            .or_default() += 1;
        if let Some(reason) = a["reason"].as_str() {
            reasons.insert(reason.to_owned());
        }
    }
    let guide = crate::reference_guides::guide(metric);
    println!(
        "{}",
        json!({"section":"reference_audit","metric":metric,"bucket_statuses":counts,"reasons":reasons,"sources":guide["citations"],"rule_count":guide["rules"].as_array().map_or(0,Vec::len)})
    );
}

pub async fn run(
    db: &Database,
    user: &str,
    date: String,
    timezone: String,
) -> Result<(), AppError> {
    let preferences = db.wellness_preferences(user).await?;
    let profile: crate::reference_guides::Profile =
        serde_json::from_value(preferences["preferences"]["reference_profile"].clone())
            .map_err(|_| AppError::Internal)?;
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
            Ok(mut v) => {
                audit_reference(db, user, &mut v, kind, &profile).await;
                println!(
                    "{}",
                    json!({"section":"full_daily_history","metric":kind,"stored":total(kind),"bars":bars(&v),"first_record_at":v["first_record_at"],"first_plotted_at":v["chart_ranges"]["D"].as_array().into_iter().flatten().filter_map(|r|r["start_at"].as_i64()).min(),"last_plotted_at":v["chart_ranges"]["D"].as_array().into_iter().flatten().filter_map(|r|r["end_at"].as_i64()).max()})
                );
            }
            Err(e) => println!(
                "{}",
                json!({"section":"full_daily_history","metric":kind,"error":e.to_string()})
            ),
        }
    }
    let end = crate::authentication::now()?;
    for definition in crate::apple_health::definitions().iter().filter(|d| {
        (d.family == "quantity" || crate::wellness::combined_chart::supports(&d.kind))
            && d.browse
            && total(&d.kind) > 0
    }) {
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
                    Ok(mut v) => {
                        if let Some(components) = v["components"].as_array_mut() {
                            for component in components {
                                let id = component["id"].as_str().unwrap_or("").to_owned();
                                audit_reference(db, user, component, &id, &profile).await;
                            }
                        } else {
                            audit_reference(db, user, &mut v, kind, &profile).await;
                        }
                        earliest = v["first_at"].as_i64();
                        println!(
                            "{}",
                            json!({"section":"measurement_period","metric":kind,"period":period,"historical":historical,"stored":total(kind),"start_at":start,"end_at":stop,"first_at":v["first_at"],"last_at":v["last_at"],"samples":v["samples_read"],"excluded":v["excluded_samples"],"complete":v["complete"],"bars":v["chart_ranges"][period].as_array().map_or(0,Vec::len)+v["components"].as_array().into_iter().flatten().map(|c|c["chart_ranges"][period].as_array().map_or(0,Vec::len)).sum::<usize>(),"components":v["components"].as_array().map_or(0,Vec::len)})
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
    for definition in crate::apple_health::definitions().iter().filter(|d| {
        d.browse
            && total(&d.kind) > 0
            && d.family != "quantity"
            && d.kind != "sleep"
            && !crate::wellness::combined_chart::supports(&d.kind)
    }) {
        let result = db
            .specialty_records(
                user,
                crate::wellness::records::ListRequest {
                    kind: definition.kind.clone(),
                    after: None,
                },
            )
            .await;
        match result {
            Ok(v) => println!(
                "{}",
                json!({"section":"specialized_record","metric":definition.kind,"family":definition.family,"stored":total(&definition.kind),"listed":v["records"].as_array().map_or(0,Vec::len),"has_more":!v["next_cursor"].is_null(),"display":if definition.kind=="electrocardiogram" {"ECG waveform and recording selector"} else if definition.family=="characteristic" {"Latest value and history"} else {"Source records"}})
            ),
            Err(e) => println!(
                "{}",
                json!({"section":"specialized_record","metric":definition.kind,"error":e.to_string()})
            ),
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
            Ok(mut v) => {
                audit_reference(db, user, &mut v, id, &profile).await;
                println!(
                    "{}",
                    json!({"section":"labs","metric":id,"name":metric["name"],"records":v["records"].as_array().map_or(0,Vec::len),"points":v["points"].as_array().map_or(0,Vec::len),"bars":bars(&v),"excluded":v["excluded_count"]})
                );
            }
            Err(e) => println!(
                "{}",
                json!({"section":"labs","metric":id,"error":e.to_string()})
            ),
        }
    }
    Ok(())
}
