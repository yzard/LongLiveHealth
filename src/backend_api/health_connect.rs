//! Read-time projection of Health Connect sleep sessions; the raw session stays intact.
use crate::{error::AppError, health::StoredHealthRecord};
use serde_json::{Value, json};

pub(crate) fn sleep_intervals(
    records: Vec<StoredHealthRecord>,
) -> Result<Vec<StoredHealthRecord>, AppError> {
    let mut result = Vec::new();
    for record in records {
        if record.platform != "health_connect" || record.record_type != "sleep" {
            result.push(record);
            continue;
        }
        let envelope: Value = serde_json::from_str(&record.payload_json)?;
        let Some(stages) = envelope["payload"]["stages"].as_array() else {
            result.push(record);
            continue;
        };
        if stages.is_empty() {
            let mut unknown = envelope.clone();
            unknown["payload"]["category"] = json!(-1);
            result.push(StoredHealthRecord {
                payload_json: serde_json::to_string(&unknown)?,
                ..record
            });
            continue;
        }
        let mut projection = envelope.clone();
        projection["payload"]
            .as_object_mut()
            .expect("Payload contains stages")
            .remove("stages");
        for stage in stages {
            let from = stage["start_at"]
                .as_i64()
                .ok_or(AppError::Invalid("Sleep stage has no start time"))?;
            let to = stage["end_at"]
                .as_i64()
                .ok_or(AppError::Invalid("Sleep stage has no end time"))?;
            if from < record.start_at || to > record.end_at || to <= from {
                return Err(AppError::Invalid("Sleep stage is outside its session"));
            }
            // The two platforms use different integer enums. Unknown is never counted as sleep.
            let category = match stage["stage"].as_i64() {
                Some(0) => -1,
                Some(1 | 3 | 7) => 2,
                Some(2) => 1,
                Some(4) => 3,
                Some(5) => 4,
                Some(6) => 5,
                _ => return Err(AppError::Invalid("Unknown Health Connect sleep stage")),
            };
            let mut projected = projection.clone();
            projected["payload"]["category"] = json!(category);
            result.push(StoredHealthRecord {
                platform: record.platform.clone(),
                source_id: record.source_id.clone(),
                record_id: record.record_id.clone(),
                record_type: record.record_type.clone(),
                start_at: from,
                end_at: to,
                version: record.version,
                payload_json: serde_json::to_string(&projected)?,
            });
        }
    }
    Ok(result)
}
