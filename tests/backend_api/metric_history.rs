use crate::{add_user, archive, fixture, observation_payload};
use base64::{Engine, engine::general_purpose::STANDARD};
use longlivehealth::{
    health::{ConnectionRequest, HealthRecordInput, SyncRequest},
    metric_history::{HistoryRequest, history},
    reports::{ReviewItem, ReviewRequest},
};
use serde_json::{Value, json};

async fn read(state: &longlivehealth::app::AppState, user: &str) -> Value {
    history(
        &state.database,
        user,
        HistoryRequest {
            metric_id: "ldl_cholesterol".into(),
        },
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn metric_history_preserves_three_sources_normalizes_and_isolates_users() {
    let (_directory, state) = fixture().await;
    let alice = add_user(&state, "alice").await;
    let bob = add_user(&state, "bob").await;
    for name in ["Function Health", "Local Clinic"] {
        let report = archive(&state, "alice").await;
        let response = state
            .database
            .review(
                &alice.user_id,
                ReviewRequest {
                    report_id: report,
                    expected_revision: 1,
                    context: Some(serde_json::from_value(json!({"source_name":name})).unwrap()),
                    observations: vec![
                        ReviewItem {
                            observation_id: None,
                            expected_revision: None,
                            status: "confirmed".into(),
                            payload: observation_payload(),
                        },
                        ReviewItem {
                            observation_id: None,
                            expected_revision: None,
                            status: "pending".into(),
                            payload: observation_payload(),
                        },
                    ],
                },
            )
            .await
            .unwrap();
        assert_eq!(response["context"]["source_name"], name);
    }
    let connection = state
        .database
        .create_connection(
            &alice.user_id,
            ConnectionRequest {
                platform: "apple_health".into(),
                installation_id: uuid::Uuid::new_v4().to_string(),
            },
        )
        .await
        .unwrap();
    let fhir = json!({"resourceType":"Observation","status":"final", "effectiveDateTime":"2026-08-02T12:00:00Z",
        "code":{"coding":[{"system":"http://loinc.org","code":"2089-1"}]},
        "valueQuantity":{"value":3.0,"code":"mmol/L","system":"http://unitsofmeasure.org"}});
    let record = |id: &str, resource: &Value| HealthRecordInput {
        record_id: id.into(),
        source_id: "clinical-provider".into(),
        record_type: "HKClinicalTypeIdentifierLabResultRecord".into(),
        start_at: 1785672000,
        end_at: 1785672000,
        version: 1,
        deleted: false,
        payload: json!({"source_name":"Clinic through Health", "fhir":{"data_base64":STANDARD.encode(serde_json::to_vec(resource).unwrap())}}),
    };
    let mut preliminary = fhir.clone();
    preliminary["status"] = json!("preliminary");
    let mut missing_date = fhir.clone();
    missing_date
        .as_object_mut()
        .unwrap()
        .remove("effectiveDateTime");
    let mut bounded = fhir.clone();
    bounded["valueQuantity"]["comparator"] = json!("<");
    let request = SyncRequest {
        connection_id: connection.connection_id.clone(),
        batch_id: uuid::Uuid::new_v4().to_string(),
        record_type: "HKClinicalTypeIdentifierLabResultRecord".into(),
        coverage_status: "observed".into(),
        records: vec![
            record("final", &fhir),
            record("preliminary", &preliminary),
            record("missing-date", &missing_date),
            record("bounded", &bounded),
        ],
    };
    let replay = serde_json::to_vec(&request).unwrap();
    state
        .database
        .sync_health(&alice.user_id, request)
        .await
        .unwrap();
    state
        .database
        .sync_health(&alice.user_id, serde_json::from_slice(&replay).unwrap())
        .await
        .unwrap();
    let result = read(&state, &alice.user_id).await;
    let points = result["points"].as_array().unwrap();
    assert_eq!(points.len(), 3);
    assert_eq!(result["excluded_count"], 3);
    let names: std::collections::BTreeSet<_> = points
        .iter()
        .map(|p| p["source_name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        ["Clinic through Health", "Function Health", "Local Clinic"]
            .into_iter()
            .collect()
    );
    let health = points
        .iter()
        .find(|p| p["channel"] == "Apple Health")
        .unwrap();
    let value: f64 = health["value"].as_str().unwrap().parse().unwrap();
    assert!((value - 116.01).abs() < 0.001);
    assert_eq!(
        health["original"]["clinical_observation"]["valueQuantity"]["value"],
        3.0
    );
    assert!(
        read(&state, &bob.user_id).await["points"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let mut deleted = record("final", &fhir);
    deleted.deleted = true;
    deleted.version = 2;
    state
        .database
        .sync_health(
            &alice.user_id,
            SyncRequest {
                connection_id: connection.connection_id,
                batch_id: uuid::Uuid::new_v4().to_string(),
                record_type: "HKClinicalTypeIdentifierLabResultRecord".into(),
                coverage_status: "observed".into(),
                records: vec![deleted],
            },
        )
        .await
        .unwrap();
    assert_eq!(
        read(&state, &alice.user_id).await["points"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn report_source_rejects_empty_or_reserved_names_without_changing_revision() {
    let (_directory, state) = fixture().await;
    let alice = add_user(&state, "alice").await;
    let report = archive(&state, "alice").await;
    for name in [" ", "Apple Health", "clinic\nother"] {
        let result = state
            .database
            .review(
                &alice.user_id,
                ReviewRequest {
                    report_id: report.clone(),
                    expected_revision: 1,
                    observations: vec![],
                    context: Some(serde_json::from_value(json!({"source_name":name})).unwrap()),
                },
            )
            .await;
        assert!(matches!(
            result,
            Err(longlivehealth::error::AppError::Invalid(_))
        ));
    }
    assert_eq!(
        state
            .database
            .report_summary(&alice.user_id, &report)
            .await
            .unwrap()
            .revision,
        1
    );
}

#[tokio::test]
async fn quantity_glucose_and_clinical_component_share_report_units_and_dates() {
    let (_directory, state) = fixture().await;
    let alice = add_user(&state, "alice").await;
    let report = archive(&state, "alice").await;
    let mut payload = observation_payload();
    payload.metric_id = Some("glucose".into());
    payload.raw_name = "Glucose".into();
    payload.raw_result = "5.5".into();
    payload.raw_unit = Some("mmol/L".into());
    state
        .database
        .review(
            &alice.user_id,
            ReviewRequest {
                report_id: report.clone(),
                expected_revision: 1,
                context: Some(serde_json::from_value(json!({"source_name":"Clinic"})).unwrap()),
                observations: vec![ReviewItem {
                    observation_id: None,
                    expected_revision: None,
                    status: "confirmed".into(),
                    payload,
                }],
            },
        )
        .await
        .unwrap();
    let connection = state
        .database
        .create_connection(
            &alice.user_id,
            ConnectionRequest {
                platform: "apple_health".into(),
                installation_id: uuid::Uuid::new_v4().to_string(),
            },
        )
        .await
        .unwrap();
    let panel = json!({"resourceType":"Observation","status":"final","effectiveDateTime":"2026-08-03T00:00:00Z",
        "component":[{"code":{"coding":[{"system":"http://loinc.org","code":"2345-7"}]},
            "valueQuantity":{"value":5.5,"code":"mmol/L","system":"http://unitsofmeasure.org"}}]});
    for (kind, payload) in [
        (
            "blood_glucose",
            json!({"value":99.1001,"unit":"mg/dL","source_name":"Meter"}),
        ),
        (
            "HKClinicalTypeIdentifierLabResultRecord",
            json!({"fhir":{"data_base64":STANDARD.encode(serde_json::to_vec(&panel).unwrap())}}),
        ),
    ] {
        state
            .database
            .sync_health(
                &alice.user_id,
                SyncRequest {
                    connection_id: connection.connection_id.clone(),
                    batch_id: uuid::Uuid::new_v4().to_string(),
                    record_type: kind.into(),
                    coverage_status: "observed".into(),
                    records: vec![HealthRecordInput {
                        record_id: kind.into(),
                        source_id: "health".into(),
                        record_type: kind.into(),
                        start_at: 1785672000,
                        end_at: 1785672000,
                        version: 1,
                        deleted: false,
                        payload,
                    }],
                },
            )
            .await
            .unwrap();
    }
    state
        .database
        .review(
            &alice.user_id,
            ReviewRequest {
                report_id: report,
                expected_revision: 2,
                observations: vec![],
                context: Some(
                    serde_json::from_value(json!({"source_name":"Renamed Clinic"})).unwrap(),
                ),
            },
        )
        .await
        .unwrap();
    let result = history(
        &state.database,
        &alice.user_id,
        HistoryRequest {
            metric_id: "glucose".into(),
        },
    )
    .await
    .unwrap();
    let points = result["points"].as_array().unwrap();
    assert_eq!(points.len(), 3);
    assert_eq!(result["excluded_count"], 0);
    assert!(points.iter().all(|p| p["unit"] == "mg/dL"));
    assert_eq!(
        points
            .iter()
            .filter(|p| p["source_name"] == "Apple Health")
            .count(),
        2
    );
    assert_eq!(
        points
            .iter()
            .filter(|p| p["source_name"] == "Renamed Clinic")
            .count(),
        1
    );
    let component = points
        .iter()
        .find(|p| p["point_id"].as_str().unwrap().ends_with(":component:0"))
        .unwrap();
    assert_eq!(component["sampled_at"], "2026-08-03T00:00:00Z");
    assert_eq!(
        component["original"]["clinical_observation"]["valueQuantity"]["value"],
        5.5
    );
}

#[tokio::test]
async fn every_catalog_metric_plots_its_own_records_without_unrelated_exclusions() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "chart-matrix").await;
    let connection = state
        .database
        .create_connection(
            &user.user_id,
            ConnectionRequest {
                platform: "apple_health".into(),
                installation_id: uuid::Uuid::new_v4().to_string(),
            },
        )
        .await
        .unwrap();
    let codes = [
        "2093-3", "2089-1", "2085-9", "2571-8", "1884-6", "4548-4", "2345-7", "718-7", "2160-0",
        "2276-4", "1751-7", "1988-5", "736-9", "787-2", "788-0", "6768-6", "6690-2",
    ];
    let metrics = longlivehealth::laboratory::metrics();
    assert_eq!(codes.len(), metrics.len());
    let mut records = Vec::new();
    for (index, metric) in metrics.iter().enumerate() {
        let resource = json!({"resourceType":"Observation","status":"final","effectiveDateTime":"2026-08-03T00:00:00Z",
            "code":{"coding":[{"system":"http://loinc.org","code":codes[index]}]},
            "valueQuantity":{"value":5,"code":if metric.metric_id == "wbc" { "K/uL" } else { metric.standard_unit },"system":"http://unitsofmeasure.org"}});
        records.push(HealthRecordInput { record_id: metric.metric_id.into(), source_id:"synthetic".into(),
            record_type:"HKClinicalTypeIdentifierLabResultRecord".into(), start_at:1785715200, end_at:1785715200,
            version:1, deleted:false, payload:json!({"fhir":{"data_base64":STANDARD.encode(serde_json::to_vec(&resource).unwrap())}}) });
    }
    let mut unknown = records[0].clone();
    unknown.record_id = "unknown".into();
    unknown.payload = json!({"fhir":{"data_base64":STANDARD.encode(serde_json::to_vec(&json!({"resourceType":"Observation", "code":{"coding":[{"system":"http://loinc.org","code":"unmapped-test-code"}]},"valueQuantity":{"value":10,"unit":"mg/dL"}})).unwrap())}});
    records.push(unknown);
    let mut unreadable = records[0].clone();
    unreadable.record_id = "unreadable".into();
    unreadable.payload = json!({"fhir":{"data_base64":"invalid"}});
    records.push(unreadable);
    state
        .database
        .sync_health(
            &user.user_id,
            SyncRequest {
                connection_id: connection.connection_id,
                batch_id: uuid::Uuid::new_v4().to_string(),
                record_type: "HKClinicalTypeIdentifierLabResultRecord".into(),
                coverage_status: "observed".into(),
                records,
            },
        )
        .await
        .unwrap();
    for metric in metrics {
        let result = history(
            &state.database,
            &user.user_id,
            HistoryRequest {
                metric_id: metric.metric_id.into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(
            result["points"].as_array().unwrap().len(),
            1,
            "{}",
            metric.metric_id
        );
        assert_eq!(result["points"][0]["unit"], metric.standard_unit);
        assert_eq!(result["points"][0]["value"], "5");
        assert_eq!(result["excluded_count"], 0, "{}", metric.metric_id);
        assert_eq!(result["unassigned_clinical_count"], 2);
    }
}

#[tokio::test]
async fn browse_discovers_clinical_items_groups_codes_and_retains_text_and_bounds() {
    let (_directory, state) = fixture().await;
    let alice = add_user(&state, "clinical-browse").await;
    let bob = add_user(&state, "other-browse").await;
    let connection = state
        .database
        .create_connection(
            &alice.user_id,
            ConnectionRequest {
                platform: "apple_health".into(),
                installation_id: uuid::Uuid::new_v4().to_string(),
            },
        )
        .await
        .unwrap();
    let base = json!({"resourceType":"Observation","status":"final","effectiveDateTime":"2026-08-03T00:00:00Z",
        "code":{"text":"Calcium","coding":[{"system":"http://loinc.org","code":"17861-6","display":"Calcium"}]},
        "valueQuantity":{"value":9.5,"unit":"mg/dL","system":"http://unitsofmeasure.org"}});
    let mut resources = vec![
        ("one", "Hospital", base.clone()),
        ("two", "Other Hospital", base.clone()),
    ];
    let mut molar = base.clone();
    molar["valueQuantity"]["unit"] = json!("mmol/L");
    resources.push(("molar", "Hospital", molar));
    let mut text = base.clone();
    text.as_object_mut().unwrap().remove("valueQuantity");
    text["valueString"] = json!("See note");
    resources.push(("text", "Hospital", text));
    let mut bound = base.clone();
    bound["valueQuantity"]["comparator"] = json!("<");
    resources.push(("bound", "Hospital", bound));
    let mut preliminary = base.clone();
    preliminary["status"] = json!("preliminary");
    resources.push(("preliminary", "Hospital", preliminary));
    let mut local = base.clone();
    local["code"]["coding"] = json!([{"system":"https://example.org/local","code":"1"}]);
    resources.push(("local-one", "Hospital", local.clone()));
    resources.push(("local-two", "Other Hospital", local));
    let records = resources.into_iter().map(|(id,source,r)|HealthRecordInput { record_id:id.into(),source_id:source.into(),record_type:"HKClinicalTypeIdentifierLabResultRecord".into(),start_at:1785715200,end_at:1785715200,version:1,deleted:false,payload:json!({"source_name":source,"fhir":{"data_base64":STANDARD.encode(serde_json::to_vec(&r).unwrap())}})}).collect();
    state
        .database
        .sync_health(
            &alice.user_id,
            SyncRequest {
                connection_id: connection.connection_id,
                batch_id: uuid::Uuid::new_v4().to_string(),
                record_type: "HKClinicalTypeIdentifierLabResultRecord".into(),
                coverage_status: "observed".into(),
                records,
            },
        )
        .await
        .unwrap();
    let catalog = longlivehealth::metric_history::browse(&state.database, &alice.user_id)
        .await
        .unwrap();
    let entries = catalog["metrics"].as_array().unwrap();
    assert_eq!(entries.len(), 3); // LOINC across hospitals, local codes scoped to each source.
    let calcium = entries.iter().find(|m| m["record_count"] == 6).unwrap();
    let id = calcium["metric_id"].as_str().unwrap();
    let result = history(
        &state.database,
        &alice.user_id,
        HistoryRequest {
            metric_id: id.into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(result["records"].as_array().unwrap().len(), 6);
    assert_eq!(result["points"].as_array().unwrap().len(), 3);
    assert!(
        result["records"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["display_value"] == "See note")
    );
    assert!(
        result["records"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["display_value"] == "<9.5")
    );
    assert!(
        result["points"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["unit"] == "mmol/L")
    );
    assert!(
        longlivehealth::metric_history::browse(&state.database, &bob.user_id)
            .await
            .unwrap()["metrics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        history(
            &state.database,
            &bob.user_id,
            HistoryRequest {
                metric_id: id.into()
            }
        )
        .await,
        Err(longlivehealth::error::AppError::NotFound)
    ));
}

#[tokio::test]
async fn equivalent_clinical_units_coalesce_display_but_keep_originals_and_distinct_results() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "unit-grouping").await;
    let connection = state
        .database
        .create_connection(
            &user.user_id,
            ConnectionRequest {
                platform: "apple_health".into(),
                installation_id: uuid::Uuid::new_v4().to_string(),
            },
        )
        .await
        .unwrap();
    let cases = [
        (
            "basophils",
            "704-7",
            "Absolute Basophil Count",
            vec!["x10(3)/uL", "x10^9/L", "10*9/L"],
            "10^9/L",
        ),
        (
            "thyroglobulin",
            "3013-0",
            "Thyroglobulin",
            vec!["NG/mL", "ng/mL", "NG/ML"],
            "ng/mL",
        ),
        (
            "wbc",
            "6690-2",
            "White blood cell count",
            vec!["K/uL", "x10(3)/uL", "10^9/L"],
            "10^9/L",
        ),
    ];
    let mut records = Vec::new();
    for (prefix, code, name, units, _) in &cases {
        for (index, unit) in units.iter().enumerate() {
            let resource = json!({"resourceType":"Observation","status":"final","effectiveDateTime":"2026-08-03T00:00:00Z",
                "code":{"text":name,"coding":[{"system":"http://loinc.org","code":code}]},
                "valueQuantity":{"value":0.04,"unit":unit,"system":"http://unitsofmeasure.org"}});
            records.push(HealthRecordInput { record_id:format!("{prefix}-{index}"),source_id:"Hospital".into(),record_type:"HKClinicalTypeIdentifierLabResultRecord".into(),start_at:1785715200,end_at:1785715200,version:1,deleted:false,payload:json!({"source_name":"Hospital","fhir":{"data_base64":STANDARD.encode(serde_json::to_vec(&resource).unwrap())}})});
            if index == 0 {
                for variant in ["other-source", "other-value", "other-method"] {
                    let mut resource = resource.clone();
                    if variant == "other-value" {
                        resource["valueQuantity"]["value"] = json!(0.05);
                    }
                    if variant == "other-method" {
                        resource["method"] = json!({"text":"Different assay"});
                    }
                    records.push(HealthRecordInput { record_id:format!("{prefix}-{variant}"),source_id:if variant == "other-source" {"Other Hospital"} else {"Hospital"}.into(),record_type:"HKClinicalTypeIdentifierLabResultRecord".into(),start_at:1785715200,end_at:1785715200,version:1,deleted:false,payload:json!({"source_name":"Hospital","fhir":{"data_base64":STANDARD.encode(serde_json::to_vec(&resource).unwrap())}})});
                }
            }
        }
    }
    state
        .database
        .sync_health(
            &user.user_id,
            SyncRequest {
                connection_id: connection.connection_id,
                batch_id: uuid::Uuid::new_v4().to_string(),
                record_type: "HKClinicalTypeIdentifierLabResultRecord".into(),
                coverage_status: "observed".into(),
                records,
            },
        )
        .await
        .unwrap();
    let catalog = longlivehealth::metric_history::browse(&state.database, &user.user_id)
        .await
        .unwrap();
    for (_, _, name, _, unit) in cases {
        let metric = catalog["metrics"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["name"] == name)
            .unwrap();
        let result = history(
            &state.database,
            &user.user_id,
            HistoryRequest {
                metric_id: metric["metric_id"].as_str().unwrap().into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(result["records"].as_array().unwrap().len(), 4, "{name}");
        assert_eq!(result["points"].as_array().unwrap().len(), 4, "{name}");
        assert!(
            result["points"]
                .as_array()
                .unwrap()
                .iter()
                .all(|p| p["unit"] == unit)
        );
        let merged = result["records"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["original_records"].as_array().unwrap().len() == 3)
            .unwrap();
        assert_eq!(merged["value"], "0.04");
        assert_eq!(
            merged["original_records"][0]["observation"]["valueQuantity"]["value"],
            0.04
        );
    }
}
