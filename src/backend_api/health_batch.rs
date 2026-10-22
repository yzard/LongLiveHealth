//! Validate and encode a health batch before acquiring its SQLite write transaction.
use crate::{
    error::AppError,
    health::{HealthRecordInput, SyncRequest},
};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, io::Write};

pub const MAX_BATCH_BYTES: usize = 40 * 1024 * 1024;
pub const MAX_PAYLOAD_BYTES: usize = 32 * 1024 * 1024;

pub(crate) struct PreparedRecord {
    pub record_id: String,
    pub source_id: String,
    pub record_type: String,
    pub start_at: i64,
    pub end_at: i64,
    pub version: i64,
    pub deleted: bool,
    pub payload_json: String,
    pub raw_json: String,
}
pub(crate) struct PreparedBatch {
    pub connection_id: String,
    pub batch_id: String,
    pub record_type: String,
    pub coverage_status: String,
    pub records: Vec<PreparedRecord>,
    pub digest: String,
}
struct BoundedDigest {
    bytes: usize,
    limit: usize,
    exceeded: bool,
    hash: Sha256,
}
impl BoundedDigest {
    fn new(limit: usize) -> Self {
        Self {
            bytes: 0,
            limit,
            exceeded: false,
            hash: Sha256::new(),
        }
    }
    fn encode<T: serde::Serialize>(&mut self, value: &T) -> Result<(), AppError> {
        if serde_json::to_writer(&mut *self, value).is_err() {
            return Err(if self.exceeded {
                AppError::TooLarge
            } else {
                AppError::Internal
            });
        }
        Ok(())
    }
}
impl Write for BoundedDigest {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.bytes) {
            self.exceeded = true;
            return Err(std::io::Error::other("JSON limit exceeded"));
        }
        self.bytes += bytes.len();
        self.hash.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
pub(crate) fn prepare(request: SyncRequest) -> Result<PreparedBatch, AppError> {
    if uuid::Uuid::parse_str(&request.batch_id).is_err()
        || request.records.len() > 1000
        || request.record_type.is_empty()
        || request.record_type.len() > 128
        || !["observed", "no_visible_samples", "error", "unsupported"]
            .contains(&request.coverage_status.as_str())
    {
        return Err(AppError::Invalid("Invalid sync batch"));
    }
    // Streaming serialization computes the existing canonical digest without a second
    // full 40 MiB batch allocation. No raw field is normalized or removed from the digest.
    let mut batch_digest = BoundedDigest::new(MAX_BATCH_BYTES);
    batch_digest.encode(&request)?;
    let digest = hex::encode(batch_digest.hash.finalize());
    let mut seen = HashSet::new();
    let mut records = Vec::with_capacity(request.records.len());
    for record in request.records {
        if record.record_id.is_empty()
            || record.record_id.len() > 256
            || record.source_id.is_empty()
            || record.source_id.len() > 512
            || (!record.deleted && record.source_id == "*")
            || record.record_type != request.record_type
            || record.version < 1
            || record.end_at < record.start_at
            || !record.payload.is_object()
            || !seen.insert((record.source_id.clone(), record.record_id.clone()))
        {
            return Err(AppError::Invalid("Invalid health record"));
        }
        BoundedDigest::new(MAX_PAYLOAD_BYTES).encode(&record.payload)?;
        let raw_json = if record.deleted {
            "{}".to_owned()
        } else {
            serde_json::to_string(&record)?
        };
        let payload_json = project_json(&raw_json)?;
        let HealthRecordInput {
            record_id,
            source_id,
            record_type,
            start_at,
            end_at,
            version,
            deleted,
            payload: _,
        } = record;
        records.push(PreparedRecord {
            record_id,
            source_id,
            record_type,
            start_at,
            end_at,
            version,
            deleted,
            payload_json,
            raw_json,
        });
    }
    Ok(PreparedBatch {
        connection_id: request.connection_id,
        batch_id: request.batch_id,
        record_type: request.record_type,
        coverage_status: request.coverage_status,
        records,
        digest,
    })
}

/// Keep one query projection; complete provider data lives only in source files.
/// The semantic digest still detects same-version changes to omitted detail fields.
pub(crate) fn project_json(text: &str) -> Result<String, AppError> {
    use serde_json::{Value, json};
    let mut record: Value = serde_json::from_str(text)?;
    if record.get("content_hash").and_then(Value::as_str).is_some() {
        return Ok(serde_json::to_string(&record)?);
    }
    if record.as_object().is_some_and(|r| r.is_empty()) {
        return Ok("{}".into());
    }
    let payload = record
        .get_mut("payload")
        .and_then(Value::as_object_mut)
        .ok_or(AppError::Internal)?;
    let apple = payload
        .get("raw_archive")
        .and_then(|r| r.get("format"))
        .and_then(Value::as_str)
        == Some("nskeyedarchiver-secure-v1");
    if apple {
        payload.remove("raw_archive");
    }
    let mut hash = Sha256::new();
    hash_value(&record, &mut hash)?;
    let content_hash = hex::encode(hash.finalize());
    let mut payload = record["payload"].clone();
    if apple
        && record["record_type"] == "heart_rate"
        && let Some(samples) = heart_samples(&payload)
    {
        payload["quantity_samples"] = serde_json::to_value(samples)?;
    }
    if apple {
        let fields = payload.as_object_mut().ok_or(AppError::Internal)?;
        for name in [
            "series",
            "voltage_measurements",
            "device",
            "metadata",
            "source_version",
            "platform_identifier",
            "start_at_precise",
            "end_at_precise",
            "raw_quantity",
        ] {
            fields.remove(name);
        }
    }
    Ok(serde_json::to_string(
        &json!({"payload":payload,"content_hash":content_hash}),
    )?)
}

// A language-independent digest encoding shared with the offline importer:
// tagged values, big-endian lengths/IEEE754 numbers, sorted UTF-8 object keys.
fn hash_value(value: &serde_json::Value, hash: &mut Sha256) -> Result<(), AppError> {
    use serde_json::Value;
    match value {
        Value::Null => hash.update(b"n"),
        Value::Bool(v) => hash.update(if *v { b"t" } else { b"f" }),
        Value::Number(v) => {
            if v.is_i64() || v.is_u64() {
                hash.update(b"i");
                let text = v.to_string();
                hash.update((text.len() as u64).to_be_bytes());
                hash.update(text.as_bytes());
            } else {
                hash.update(b"d");
                hash.update(v.as_f64().ok_or(AppError::Internal)?.to_be_bytes());
            }
        }
        Value::String(v) => {
            hash.update(b"s");
            hash.update((v.len() as u64).to_be_bytes());
            hash.update(v.as_bytes());
        }
        Value::Array(values) => {
            hash.update(b"a");
            hash.update((values.len() as u64).to_be_bytes());
            for v in values {
                hash_value(v, hash)?;
            }
        }
        Value::Object(values) => {
            hash.update(b"o");
            hash.update((values.len() as u64).to_be_bytes());
            let mut keys = values.keys().collect::<Vec<_>>();
            keys.sort();
            for key in keys {
                hash_value(&Value::String(key.clone()), hash)?;
                hash_value(&values[key], hash)?;
            }
        }
    }
    Ok(())
}

/// HealthKit source display names can change independently of immutable samples.
/// Restore only that label before comparing the entire original semantic digest.
pub(crate) fn only_source_name_changed(previous: &str, incoming: &str) -> Result<bool, AppError> {
    let previous_value: serde_json::Value = serde_json::from_str(previous)?;
    let mut incoming: serde_json::Value = serde_json::from_str(incoming)?;
    let Some(payload) = incoming["payload"].as_object_mut() else {
        return Ok(false);
    };
    match previous_value["payload"].get("source_name") {
        Some(name) => {
            payload.insert("source_name".into(), name.clone());
        }
        None => {
            payload.remove("source_name");
        }
    }
    same_content(previous, &project_json(&serde_json::to_string(&incoming)?)?)
}

/// Small query projection only; opaque series archives stay in raw files.
pub(crate) fn heart_samples(payload: &serde_json::Value) -> Option<Vec<serde_json::Value>> {
    use serde_json::json;
    let series = payload["series"].as_array()?;
    let samples: Option<Vec<_>> = series
        .iter()
        .map(|sample| {
            let at = sample["start_at"].as_f64()?;
            let text = sample["quantity"].as_str()?;
            let (value, unit) = text.split_once(' ')?;
            let value = value.parse::<f64>().ok()?
                * match unit.trim() {
                    "count/s" => 60.0,
                    "count/min" | "bpm" => 1.0,
                    _ => return None,
                };
            (at.is_finite() && value.is_finite() && value > 0.0 && value <= 300.0)
                .then(|| json!({"at":at,"value":value}))
        })
        .collect();
    samples.filter(|samples| !samples.is_empty())
}

pub(crate) fn same_content(first: &str, second: &str) -> Result<bool, AppError> {
    let a: serde_json::Value = serde_json::from_str(&project_json(first)?)?;
    let b: serde_json::Value = serde_json::from_str(&project_json(second)?)?;
    Ok(a == b || (a["content_hash"].is_string() && a["content_hash"] == b["content_hash"]))
}
