//! Combine disjoint step intervals; select a device only over overlapping time.
use crate::{error::AppError, health::StoredHealthRecord};
use serde_json::{Value, json};
use std::collections::BTreeMap;

type Source = (u8, String);
#[derive(Default)]
struct Boundary {
    changes: Vec<(Source, f64, i32)>,
    points: BTreeMap<Source, f64>,
}

pub fn device_priority(payload: &Value) -> u8 {
    let device = &payload["device"];
    let hardware = format!(
        "{} {} {}",
        device["name"].as_str().unwrap_or(""),
        device["manufacturer"].as_str().unwrap_or(""),
        device["model"].as_str().unwrap_or("")
    )
    .to_lowercase();
    let name = payload["source_name"].as_str().unwrap_or("").to_lowercase();
    let classify = |text: &str| {
        if text.contains("apple watch")
            || text.contains("applewatch")
            || (text.contains("apple") && text.contains("watch"))
        {
            return 0;
        }
        if ["iphone", "phone", "手机"]
            .iter()
            .any(|word| text.contains(word))
        {
            return 3;
        }
        if [
            "watch",
            "fitbit",
            "garmin",
            "polar",
            "wahoo",
            "whoop",
            "oura",
            "huawei",
            "华为",
            "手表",
            "手环",
            "心率带",
            "wrist",
            "chest strap",
            "heart rate strap",
        ]
        .iter()
        .any(|word| text.contains(word))
        {
            return 1;
        }
        2
    };
    let rank = classify(&hardware);
    if rank == 2 { classify(&name) } else { rank }
}

pub fn combined_day(
    records: &[StoredHealthRecord],
    lower: i64,
    upper: i64,
) -> Result<Value, AppError> {
    let mut boundaries: BTreeMap<i64, Boundary> = BTreeMap::new();
    let mut valid = 0;
    for record in records {
        if record.end_at < lower || record.start_at >= upper || record.end_at < record.start_at {
            continue;
        }
        let envelope: Value = serde_json::from_str(&record.payload_json)?;
        let payload = &envelope["payload"];
        let Some(value) = payload["value"]
            .as_f64()
            .filter(|v| v.is_finite() && *v >= 0.0)
        else {
            continue;
        };
        if payload["unit"] != "count" {
            continue;
        }
        let source = (
            device_priority(payload),
            format!("{}:{}", record.platform, record.source_id),
        );
        if record.start_at == record.end_at {
            if record.start_at >= lower {
                *boundaries
                    .entry(record.start_at)
                    .or_default()
                    .points
                    .entry(source)
                    .or_default() += value;
                valid += 1;
            }
        } else {
            let from = record.start_at.max(lower);
            let to = record.end_at.min(upper);
            if from >= to {
                continue;
            }
            let rate = value / (i128::from(record.end_at) - i128::from(record.start_at)) as f64;
            boundaries
                .entry(from)
                .or_default()
                .changes
                .push((source.clone(), rate, 1));
            boundaries
                .entry(to)
                .or_default()
                .changes
                .push((source, -rate, -1));
            valid += 1;
        }
    }
    let mut active: BTreeMap<Source, (f64, i32)> = BTreeMap::new();
    let mut contributions: BTreeMap<String, f64> = BTreeMap::new();
    let mut previous = lower;
    for (at, boundary) in boundaries {
        if let Some(((_, source), (rate, _))) = active.first_key_value() {
            *contributions.entry(source.clone()).or_default() +=
                rate.max(0.0) * (at - previous) as f64;
        }
        for (source, delta, count) in boundary.changes {
            let entry = active.entry(source.clone()).or_default();
            entry.0 += delta;
            entry.1 += count;
            if entry.1 == 0 {
                active.remove(&source);
            }
        }
        if let Some((source, value)) = boundary.points.first_key_value()
            && active
                .first_key_value()
                .is_none_or(|(current, _)| source <= current)
        {
            *contributions.entry(source.1.clone()).or_default() += value;
        }
        previous = at;
    }
    let total: f64 = contributions.values().sum();
    let value = if valid > 0 && total.is_finite() {
        Some(total)
    } else {
        None
    };
    Ok(
        json!({"value":value,"unit":"count","algorithm":"overlap_device_priority_prorated_v1","contributions":contributions,
        "notes":["Disjoint intervals add together. Overlapping intervals use Apple Watch, other wearables, unknown sources, then phones.","Partial intervals allocate steps in proportion to duration; exact step timestamps are unavailable.","Zero-duration records compete at their timestamp. Same-source records retain the source's summed samples."]}),
    )
}
