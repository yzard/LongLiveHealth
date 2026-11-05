//! One account-scoped catalog for every mobile Browse entry and category.
use crate::{database::Database, error::AppError};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::OnceLock};

fn taxonomy() -> &'static Value {
    static DATA: OnceLock<Value> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("browse_categories.json"))
            .expect("validated Browse taxonomy")
    })
}
pub(crate) fn classify(kind: &str, title: &str, laboratory: bool) -> String {
    let normalize = |s: &str| {
        s.to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
    };
    let key = normalize(kind);
    let title = normalize(title);
    if laboratory {
        return if [
            "atrialrate",
            "ventricularrate",
            "qrsduration",
            "qtc",
            "qtinterval",
            "printerval",
            "paxis",
            "raxis",
            "taxis",
        ]
        .iter()
        .any(|name| *name == key || *name == title)
        {
            "heart"
        } else {
            "bloodWork"
        }
        .into();
    }
    for rule in taxonomy()["rules"].as_array().expect("rules") {
        if rule["terms"].as_array().expect("terms").iter().any(|term| {
            let term = term.as_str().expect("term");
            key.contains(term) || title.contains(term)
        }) {
            return rule["category"].as_str().expect("category").into();
        }
    }
    if key.starts_with("hkcategorytypeidentifier")
        && !key.contains("handwashing")
        && !key.contains("toothbrushing")
    {
        "symptoms"
    } else {
        "other"
    }
    .into()
}

pub fn assemble(sources: &[Value], labs: &[Value]) -> Value {
    let mut counts = BTreeMap::<String, i64>::new();
    for source in sources {
        let kind = source["record_type"].as_str().unwrap_or("");
        // Manual weight is the same measurement series as device body mass.
        let kind = match kind {
            "weight" => "body_mass",
            "blood_pressure_systolic" | "blood_pressure_diastolic" | "blood_pressure" => {
                "HKCorrelationTypeIdentifierBloodPressure"
            }
            _ => kind,
        };
        *counts.entry(kind.into()).or_default() += source["record_count"].as_i64().unwrap_or(0);
    }
    let mut entries = Vec::new();
    for (kind, count) in counts {
        if count <= 0 {
            continue;
        }
        let mut definition = if let Some(d) = crate::apple_health::definition(&kind) {
            if !d.browse {
                continue;
            }
            serde_json::to_value(d).expect("definition")
        } else if kind == "hrv_rmssd" {
            json!({"kind":kind,"title":"HRV RMSSD","family":"quantity","unit":"ms","description":"Root mean square of successive heartbeat interval differences."})
        } else {
            continue;
        };
        if let Some(title) = taxonomy()["titles"].get(&kind) {
            definition["title"] = title.clone();
        }
        if kind == "workout" {
            definition["unit"] = json!("s");
        }
        let presentation = if kind == "sleep" {
            "sleep"
        } else if crate::wellness::combined_chart::supports(&kind) {
            "measurement"
        } else if [
            "steps",
            "active_energy",
            "basal_energy",
            "resting_heart_rate",
            "hrv_sdnn",
        ]
        .contains(&kind.as_str())
        {
            "daily"
        } else if definition["family"] == "quantity"
            || crate::wellness::combined_chart::supports(&kind)
        {
            "measurement"
        } else {
            "records"
        };
        let title = definition["title"].as_str().unwrap_or(&kind);
        entries.push(json!({"id":format!("health:{kind}"),"kind":kind,"title":title,"category_id":classify(&kind,title,false),"presentation":presentation,"definition":definition,"record_count":count}));
    }
    for metric in labs {
        let kind = metric["metric_id"].as_str().unwrap_or("");
        let title = metric["name"].as_str().unwrap_or(kind);
        entries.push(json!({"id":format!("lab:{kind}"),"kind":kind,"title":title,"category_id":classify(kind,title,true),"presentation":"lab","definition":metric}));
    }
    entries.sort_by_key(|e| e["title"].as_str().unwrap_or("").to_lowercase());
    let categories: Vec<_> = taxonomy()["categories"]
        .as_array()
        .expect("categories")
        .iter()
        .filter(|category| {
            entries
                .iter()
                .any(|entry| entry["category_id"] == category["id"])
        })
        .cloned()
        .collect();
    json!({"categories":categories,"entries":entries})
}
pub async fn get(db: &Database, user: &str) -> Result<Value, AppError> {
    let sources = db.wellness_sources(user).await?;
    let labs = crate::metric_history::browse(db, user).await?;
    Ok(assemble(
        sources["sources"]
            .as_array()
            .ok_or(AppError::Invalid("Missing source catalog"))?,
        labs["metrics"]
            .as_array()
            .ok_or(AppError::Invalid("Missing laboratory catalog"))?,
    ))
}
