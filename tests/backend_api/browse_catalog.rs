use longlivehealth::{apple_health::definitions, browse_catalog::assemble};
use serde_json::json;
#[test]
fn complete_catalog_is_data_driven_and_cross_platform() {
    let empty = assemble(&[], &[]);
    assert!(empty["entries"].as_array().unwrap().is_empty());
    assert!(empty["categories"].as_array().unwrap().is_empty());
    let sources: Vec<_> = definitions()
        .iter()
        .filter(|d| d.browse)
        .map(|d| json!({"platform":"health_connect","record_type":d.kind,"record_count":1}))
        .collect();
    let result = assemble(
        &sources,
        &[
            json!({"metric_id":"apob","name":"Apolipoprotein B"}),
            json!({"metric_id":"future_lab","name":"New laboratory test"}),
        ],
    );
    let entries = result["entries"].as_array().unwrap();
    assert_eq!(entries.len(), sources.len()); // Two pressure components collapse into their existing paired entry.
    for entry in entries {
        assert!(
            result["categories"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["id"] == entry["category_id"])
        );
    }
    assert!(
        entries
            .iter()
            .filter(|e| e["presentation"] == "lab")
            .all(|e| e["category_id"] == "bloodWork")
    );
    let manual = assemble(
        &[
            json!({"record_type":"weight","record_count":1}),
            json!({"record_type":"hrv_rmssd","record_count":1}),
            json!({"record_type":"sleep","record_count":0}),
        ],
        &[],
    );
    assert_eq!(manual["entries"].as_array().unwrap().len(), 2);
    assert!(
        manual["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["kind"] == "body_mass" && e["presentation"] == "measurement")
    );
}

#[test]
fn paired_and_component_records_are_charted_but_ecg_remains_specialized() {
    for kind in [
        "HKCorrelationTypeIdentifierBloodPressure",
        "HKCorrelationTypeIdentifierFood",
        "workout",
        "mindful_session",
        "heartbeat_series",
        "state_of_mind",
    ] {
        let result = assemble(&[json!({"record_type":kind,"record_count":1})], &[]);
        assert_eq!(
            result["entries"][0]["presentation"], "measurement",
            "{kind}"
        );
    }
    let result = assemble(
        &[json!({"record_type":"electrocardiogram","record_count":1})],
        &[],
    );
    assert_eq!(result["entries"][0]["presentation"], "records");
}

#[test]
fn blood_pressure_has_one_entry_even_without_correlations() {
    let result = assemble(
        &[
            json!({"record_type":"blood_pressure_systolic","record_count":4}),
            json!({"record_type":"blood_pressure_diastolic","record_count":4}),
            json!({"record_type":"blood_pressure","record_count":1}),
        ],
        &[],
    );
    assert_eq!(result["entries"].as_array().unwrap().len(), 1);
    assert_eq!(
        result["entries"][0]["kind"],
        "HKCorrelationTypeIdentifierBloodPressure"
    );
    assert_eq!(result["entries"][0]["presentation"], "measurement");
}

#[test]
fn clinical_ecg_titles_are_heart_metrics_and_not_blood_work() {
    for title in [
        "Atrial Rate",
        "Ventricular Rate",
        "QRS Duration",
        "QTc",
        "Q-T Interval",
        "P-R Interval",
        "P Axis",
        "R Axis",
        "T Axis",
    ] {
        let result = assemble(
            &[],
            &[json!({"metric_id":"clinical:opaque_hash", "name":title})],
        );
        assert_eq!(result["entries"][0]["category_id"], "heart", "{title}");
    }
    let result = assemble(
        &[],
        &[json!({"metric_id":"clinical:opaque_hash", "name":"Apolipoprotein B"})],
    );
    assert_eq!(result["entries"][0]["category_id"], "bloodWork");
}
