//! Validate and encode a health batch before acquiring its SQLite write transaction.
use crate::{
    error::AppError,
    health::{HealthRecordInput, SyncRequest},
};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, io::Write};

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
    pub coverage: Vec<Coverage>,
    pub records: Vec<PreparedRecord>,
    pub digest: String,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Coverage {
    pub record_type: String,
    pub status: String,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MixedSyncRequest {
    pub connection_id: String,
    pub batch_id: String,
    pub coverage: Vec<Coverage>,
    pub records: Vec<HealthRecordInput>,
}
// Existing installed apps and durable pending requests still use the single-type wire contract.
#[derive(serde::Deserialize)]
#[serde(untagged)]
pub(crate) enum IncomingSync {
    Mixed(MixedSyncRequest),
    Single(SyncRequest),
}
impl IncomingSync {
    pub fn prepare(self) -> Result<PreparedBatch, AppError> {
        match self {
            Self::Single(request) => prepare(request),
            Self::Mixed(request) => {
                let mut digest = JsonDigest::new();
                digest.encode(&request)?;
                prepare_records(request, hex::encode(digest.hash.finalize()))
            }
        }
    }
}
/// Apple medication dose revisions can be fractional (timestamp-like). Positive
/// IEEE-754 bit patterns have the same ordering as their numeric values and fit
/// in SQLite's signed integer. Keep that sortable key internal, and archive the
/// original numeric version unchanged. Other record types keep their wire contract.
pub(crate) fn prepare_http(bytes: &[u8]) -> Result<PreparedBatch, AppError> {
    use serde_json::Value;
    let mut wire: Value =
        serde_json::from_slice(bytes).map_err(|_| AppError::Invalid("Invalid health JSON"))?;
    let has_medication = wire["records"].as_array().is_some_and(|records| {
        records
            .iter()
            .any(|record| record["record_type"] == "medication_dose_event")
    });
    if !has_medication {
        return serde_json::from_slice::<IncomingSync>(bytes)
            .map_err(|_| AppError::Invalid("Invalid health JSON"))?
            .prepare();
    }
    let mut digest = JsonDigest::new();
    digest.encode(&wire)?;
    let original_digest = hex::encode(digest.hash.finalize());
    let mut originals = Vec::new();
    for (index, record) in wire["records"]
        .as_array_mut()
        .ok_or(AppError::Invalid("Invalid health records"))?
        .iter_mut()
        .enumerate()
    {
        if record["record_type"] != "medication_dose_event" {
            continue;
        }
        let version = record["version"]
            .as_f64()
            .ok_or(AppError::Invalid("Invalid medication source version"))?;
        if !version.is_finite() || version < 1.0 {
            return Err(AppError::Invalid("Invalid medication source version"));
        }
        originals.push((index, serde_json::to_string(record)?));
        record["version"] = Value::from(
            i64::try_from(version.to_bits())
                .map_err(|_| AppError::Invalid("Invalid medication source version"))?,
        );
    }
    let mut prepared = serde_json::from_value::<IncomingSync>(wire)
        .map_err(|_| AppError::Invalid("Invalid health JSON"))?
        .prepare()?;
    for (index, original) in originals {
        let record = &mut prepared.records[index];
        record.payload_json = if record.deleted {
            "{}".into()
        } else {
            project_json(&original)?
        };
        record.raw_json = original;
    }
    prepared.digest = original_digest;
    Ok(prepared)
}

struct JsonDigest {
    hash: Sha256,
}
impl JsonDigest {
    fn new() -> Self {
        Self {
            hash: Sha256::new(),
        }
    }
    fn encode<T: serde::Serialize>(&mut self, value: &T) -> Result<(), AppError> {
        serde_json::to_writer(self, value).map_err(|_| AppError::Internal)
    }
}
impl Write for JsonDigest {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.hash.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
pub(crate) fn prepare(request: SyncRequest) -> Result<PreparedBatch, AppError> {
    let mut batch_digest = JsonDigest::new();
    batch_digest.encode(&request)?;
    let digest = hex::encode(batch_digest.hash.finalize());
    prepare_records(
        MixedSyncRequest {
            connection_id: request.connection_id,
            batch_id: request.batch_id,
            coverage: vec![Coverage {
                record_type: request.record_type,
                status: request.coverage_status,
            }],
            records: request.records,
        },
        digest,
    )
}
fn prepare_records(request: MixedSyncRequest, digest: String) -> Result<PreparedBatch, AppError> {
    let mut types = HashSet::new();
    if uuid::Uuid::parse_str(&request.batch_id).is_err() || request.coverage.is_empty() {
        return Err(AppError::Invalid("Invalid sync batch"));
    }
    for coverage in &request.coverage {
        if coverage.record_type.is_empty()
            || coverage.record_type.len() > 128
            || !types.insert(coverage.record_type.as_str())
            || !["observed", "no_visible_samples", "error", "unsupported"]
                .contains(&coverage.status.as_str())
        {
            return Err(AppError::Invalid("Invalid sync coverage"));
        }
    }
    let mut seen = HashSet::new();
    let mut records = Vec::with_capacity(request.records.len());
    for record in request.records {
        if record.record_id.is_empty()
            || record.record_id.len() > 256
            || record.source_id.is_empty()
            || record.source_id.len() > 512
            || (!record.deleted && record.source_id == "*")
            || !types.contains(record.record_type.as_str())
            || record.version < 1
            || record.end_at < record.start_at
            || !record.payload.is_object()
            || !seen.insert((record.source_id.clone(), record.record_id.clone()))
        {
            return Err(AppError::Invalid("Invalid health record"));
        }
        let raw_json = serde_json::to_string(&record)?;
        let payload_json = if record.deleted {
            "{}".to_owned()
        } else {
            project_json(&raw_json)?
        };
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
        coverage: request.coverage,
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
    let components = crate::wellness::component_projection::project(
        record["record_type"].as_str().unwrap_or(""),
        &record["payload"],
        record["start_at"].as_f64().unwrap_or(0.0),
        record["end_at"].as_f64().unwrap_or(0.0),
    );
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
    if let Some(at) = crate::wellness::component_projection::timestamp(
        record["record_type"].as_str().unwrap_or(""),
        &payload,
    ) {
        payload["chart_at"] = json!(at);
    }
    if components.as_array().is_some_and(|v| !v.is_empty()) {
        payload["chart_components"] = components;
    }
    if apple
        && record["record_type"] == "heart_rate"
        && let Some(samples) = heart_samples(&payload)
    {
        payload["quantity_samples"] = serde_json::to_value(samples)?;
    }
    if apple
        && let Some(fields) = crate::apple_health::projection(
            record["record_type"].as_str().ok_or(AppError::Internal)?,
            &payload,
        )
    {
        payload
            .as_object_mut()
            .ok_or(AppError::Internal)?
            .extend(fields.as_object().ok_or(AppError::Internal)?.clone());
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

/// Accept an adjacent floating-point representation only when the immutable
/// HealthKit secure archive is byte-identical. Integers, strings, structure, and
/// nonnumeric provenance still require exact equality.
pub(crate) fn same_archived_sample(first: &[u8], second: &str) -> Result<bool, AppError> {
    use serde_json::Value;
    let a: Value = serde_json::from_slice(first)?;
    let b: Value = serde_json::from_str(second)?;
    if a["payload"]["raw_archive"]["format"] != "nskeyedarchiver-secure-v1"
        || !a["payload"]["raw_archive"]["data"].is_string()
        || a["payload"]["raw_archive"]["data"].as_str() == Some("")
        || a["payload"]["raw_archive"] != b["payload"]["raw_archive"]
    {
        return Ok(false);
    }
    fn equivalent(a: &Value, b: &Value) -> bool {
        if a == b {
            return true;
        }
        match (a, b) {
            (Value::Number(a), Value::Number(b)) if a.is_f64() && b.is_f64() => {
                let (Some(x), Some(y)) = (a.as_f64(), b.as_f64()) else {
                    return false;
                };
                x.is_finite()
                    && y.is_finite()
                    && (x == y
                        || (x.is_sign_negative() == y.is_sign_negative()
                            && x.to_bits().abs_diff(y.to_bits()) <= 1))
            }
            (Value::Array(a), Value::Array(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(x, y)| equivalent(x, y))
            }
            (Value::Object(a), Value::Object(b)) => {
                a.len() == b.len()
                    && a.iter()
                        .all(|(k, v)| b.get(k).is_some_and(|other| equivalent(v, other)))
            }
            _ => false,
        }
    }
    Ok(equivalent(&a, &b))
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
