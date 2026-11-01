use longlivehealth::reference_guides::{self, EvaluationRequest, Profile};
use serde_json::json;
use std::collections::BTreeMap;
fn request(metric: &str, value: f64, unit: &str) -> EvaluationRequest {
    EvaluationRequest {
        metric_id: metric.into(),
        value,
        unit: unit.into(),
        measured_at: 1_767_225_600,
        context: BTreeMap::new(),
    }
}
fn male() -> Profile {
    Profile {
        birth_date: Some("1980-06-15".into()),
        reference_sex: Some("male".into()),
    }
}
#[test]
fn missing_demographics_and_incompatible_units_never_mean_normal() {
    assert_eq!(
        reference_guides::evaluate(&request("albumin", 4., "g/dL"), &Profile::default())["status"],
        "not_assessed"
    );
    assert_eq!(
        reference_guides::evaluate(&request("albumin", 40., "g/L"), &male())["status"],
        "not_assessed"
    );
    assert_eq!(
        reference_guides::evaluate(&request("albumin", 4., "g/dL"), &male())["status"],
        "in_range"
    );
    assert_eq!(
        reference_guides::evaluate(&request("albumin", f64::NAN, "g/dL"), &male())["status"],
        "not_assessed"
    );
}
#[test]
fn clinical_threshold_boundaries_and_measurement_age() {
    for (n, status) in [
        (89.99, "in_range"),
        (90., "borderline_high"),
        (120., "high"),
        (140., "very_high"),
    ] {
        assert_eq!(
            reference_guides::evaluate(&request("apob", n, "mg/dL"), &male())["status"],
            status
        );
    }
    let child = Profile {
        birth_date: Some("2010-01-01".into()),
        ..male()
    };
    assert_eq!(
        reference_guides::evaluate(&request("apob", 90., "mg/dL"), &child)["status"],
        "not_assessed"
    );
    let young = Profile {
        birth_date: Some("1995-06-15".into()),
        ..male()
    };
    assert_eq!(
        reference_guides::evaluate(&request("dhea_sulfate", 600., "µg/dL"), &young)["status"],
        "in_range"
    );
    assert_eq!(
        reference_guides::evaluate(&request("dhea_sulfate", 600., "µg/dL"), &male())["status"],
        "high"
    );
}
#[test]
fn specimen_and_fasting_are_not_inferred() {
    let mut r = request("glucose", 110., "mg/dL");
    assert_eq!(
        reference_guides::evaluate(&r, &male())["status"],
        "not_assessed"
    );
    r.context.insert("fasting".into(), "yes".into());
    r.context.insert("specimen".into(), "plasma".into());
    assert_eq!(
        reference_guides::evaluate(&r, &male())["status"],
        "increased_risk"
    );
    assert_eq!(
        reference_guides::evaluate(&request("heart_rate", 150., "count/min"), &male())["status"],
        "not_assessed"
    );
}
#[test]
fn report_interval_is_specific_to_source_time_and_value() {
    let mut history = json!({"points":[{"source_key":"report:a","timestamp":100,"value":"4","reference":{"unit":"g/dL","lower":"3","upper":"4.5","lower_inclusive":true,"upper_inclusive":true}}],"chart_ranges":{"D":[{"source":"report:a","unit":"g/dL","last_at":100,"latest_value":4},{"source":"report:b","unit":"g/dL","last_at":100,"latest_value":4}]}});
    reference_guides::annotate(&mut history, "albumin", &Profile::default());
    assert_eq!(
        history["chart_ranges"]["D"][0]["reference_assessment"]["kind"],
        "report_reference"
    );
    assert_eq!(
        history["chart_ranges"]["D"][1]["reference_assessment"]["status"],
        "not_assessed"
    );
}
#[test]
fn catalog_covers_all_registered_metrics_and_rules_have_citations() {
    let c = reference_guides::catalog();
    for m in longlivehealth::laboratory::metrics() {
        assert!(c["metrics"].get(m.metric_id).is_some(), "{}", m.metric_id);
    }
    for (id, m) in c["metrics"].as_object().unwrap() {
        let rules = m["rules"].as_array().unwrap();
        assert!(
            !rules.is_empty() || !m["reason"].as_str().unwrap().is_empty(),
            "{id}"
        );
        for r in rules {
            assert!(
                c["sources"][r["source"].as_str().unwrap()]["url"]
                    .as_str()
                    .unwrap()
                    .starts_with("https://")
            );
            if let (Some(lo), Some(hi)) = (r["lower"].as_f64(), r["upper"].as_f64()) {
                assert!(lo <= hi, "{id}");
            }
            let bands = r["bands"].as_array().unwrap();
            for pair in bands.windows(2) {
                assert_eq!(pair[0]["upper"], pair[1]["lower"], "{id}");
            }
        }
    }
}

#[tokio::test]
async fn reference_api_requires_auth_and_uses_only_current_users_profile() {
    use crate::{add_user, fixture, request as api, token};
    use axum::http::StatusCode;
    let (_dir, state) = fixture().await;
    let users = [
        add_user(&state, "alice").await,
        add_user(&state, "bob").await,
    ];
    for user in users {
        let connection = state
            .database
            .create_connection(
                &user.user_id,
                longlivehealth::health::ConnectionRequest {
                    platform: "apple_health".into(),
                    installation_id: uuid::Uuid::new_v4().to_string(),
                },
            )
            .await
            .unwrap();
        let mut record =
            crate::health_batch::record("fitness", json!({"value":42,"unit":"ml/kg*min"}));
        record.record_type = "vo2_max".into();
        record.start_at = 1_767_225_600;
        record.end_at = record.start_at + 1;
        let mut batch = crate::health_batch::batch(&connection.connection_id, vec![record]);
        batch.record_type = "vo2_max".into();
        state
            .database
            .sync_health(&user.user_id, batch)
            .await
            .unwrap();
    }
    let alice = token(&state, "alice").await;
    let bob = token(&state, "bob").await;
    let router = longlivehealth::app::create_application(state);
    assert_eq!(
        api(&router, "/api/v1/wellness/chart", None, json!({}))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let (_, old) = api(
        &router,
        "/api/v1/wellness/preferences/get",
        Some(&alice),
        json!({}),
    )
    .await;
    let mut prefs = old["preferences"].clone();
    prefs["reference_profile"] = json!({"birth_date":"1980-06-15","reference_sex":"male"});
    let (status,body)=api(&router,"/api/v1/wellness/preferences/save",Some(&alice),json!({"expected_version":old["version"],"batch_id":uuid::Uuid::new_v4(),"preferences":prefs})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let input = json!({"record_type":"vo2_max","period":"D","start_at":1767225600_i64,"end_at":1767312000_i64,"timezone":"UTC"});
    let (_, a) = api(
        &router,
        "/api/v1/wellness/chart",
        Some(&alice),
        input.clone(),
    )
    .await;
    let (_, b) = api(&router, "/api/v1/wellness/chart", Some(&bob), input).await;
    assert_ne!(
        a["chart_ranges"]["D"][0]["reference_assessment"]["status"],
        "not_assessed"
    );
    assert!(a["chart_ranges"]["D"][0]["reference_assessment"].is_object());
    assert_eq!(
        b["chart_ranges"]["D"][0]["reference_assessment"]["status"],
        "not_assessed"
    );
}
#[test]
fn pregnancy_and_fhir_reference_boundaries_are_explicit() {
    let female = Profile {
        reference_sex: Some("female".into()),
        ..male()
    };
    let mut r = request("hemoglobin", 12., "g/dL");
    assert_eq!(
        reference_guides::evaluate(&r, &female)["status"],
        "not_assessed"
    );
    r.context.insert("pregnancy".into(), "not_pregnant".into());
    assert_eq!(
        reference_guides::evaluate(&r, &female)["status"],
        "in_range"
    );
    let mut h = json!({"points":[{"source_key":"apple_health:a","timestamp":100,"value":"12","original":{"clinical_observation":{"referenceRange":[{"low":{"value":10,"unit":"g/dL"},"high":{"value":13,"unit":"g/dL"}}]}}}],"chart_ranges":{"D":[{"source":"apple_health:a","unit":"g/dL","last_at":100,"latest_value":12}]}});
    reference_guides::annotate(&mut h, "hemoglobin", &Profile::default());
    assert_eq!(
        h["chart_ranges"]["D"][0]["reference_assessment"]["status"],
        "in_range"
    );
}
#[test]
fn fractional_chart_timestamps_are_assessed_not_skipped() {
    let mut h = json!({"chart_ranges":{"D":[{"source":"a","unit":"count/min","last_at":1767225600.125,"latest_value":70.0}]}});
    reference_guides::annotate(&mut h, "resting_heart_rate", &male());
    assert_eq!(
        h["chart_ranges"]["D"][0]["reference_assessment"]["status"],
        "in_range"
    );
}
#[test]
fn inclusive_risk_cutoffs_and_birthday_boundaries_are_preserved() {
    let mut r = request("hs_crp", 3., "mg/L");
    r.context
        .insert("acute_inflammation".into(), "absent".into());
    assert_eq!(
        reference_guides::evaluate(&r, &male())["status"],
        "intermediate_risk"
    );
    r.value = 3.001;
    assert_eq!(
        reference_guides::evaluate(&r, &male())["status"],
        "high_risk"
    );
    r.value = 10.;
    assert_eq!(
        reference_guides::evaluate(&r, &male())["status"],
        "not_assessed"
    );
    let profile = Profile {
        birth_date: Some("2008-01-02".into()),
        ..male()
    };
    let mut r = request("apob", 50., "mg/dL");
    assert_eq!(
        reference_guides::evaluate(&r, &profile)["status"],
        "not_assessed"
    );
    r.measured_at += 86400;
    assert_eq!(
        reference_guides::evaluate(&r, &profile)["status"],
        "in_range"
    );
}

#[test]
fn every_published_rule_evaluates_its_population_and_boundary_values() {
    let catalog = reference_guides::catalog();
    let mut checked = 0;
    for (metric, entry) in catalog["metrics"].as_object().unwrap() {
        for rule in entry["rules"].as_array().unwrap() {
            let age = rule["min_age"].as_i64().unwrap_or(0);
            let profile = Profile {
                birth_date: Some(format!("{}-01-01", 2026 - age)),
                reference_sex: Some(rule["sex"].as_str().unwrap_or("male").into()),
            };
            let mut input = request(metric, 0., entry["unit"].as_str().unwrap());
            input.context = rule["context"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap().into()))
                .collect();
            if profile.reference_sex.as_deref() == Some("female") {
                input
                    .context
                    .insert("pregnancy".into(), "not_pregnant".into());
            }
            if rule["formula"] == "adult_bmi_to_weight" {
                input.context.insert("height_m".into(), "1".into());
            }
            let mut cases = Vec::new();
            if let Some(bands) = rule["bands"].as_array().filter(|b| !b.is_empty()) {
                for band in bands {
                    let low = band["lower"].as_f64().unwrap_or(0.);
                    let high = band["upper"].as_f64().unwrap_or(low + 10.);
                    cases.push(((low + high) / 2., band["status"].as_str().unwrap()));
                    if band["lower_inclusive"] == true {
                        cases.push((low, band["status"].as_str().unwrap()));
                    }
                    if band["upper_inclusive"] == true {
                        cases.push((high, band["status"].as_str().unwrap()));
                    }
                }
            } else {
                let lo = rule["lower"].as_f64();
                let hi = rule["upper"].as_f64();
                cases.push((
                    (lo.unwrap_or(0.) + hi.unwrap_or(lo.unwrap_or(0.) + 10.)) / 2.,
                    "in_range",
                ));
                if let Some(lo) = lo {
                    cases.push((
                        lo,
                        if rule["lower_inclusive"] == true {
                            "in_range"
                        } else {
                            "low"
                        },
                    ));
                    if lo > 0. {
                        cases.push((lo / 2., "low"));
                    }
                }
                if let Some(hi) = hi {
                    cases.push((
                        hi,
                        if rule["upper_inclusive"] == true {
                            "in_range"
                        } else {
                            "high"
                        },
                    ));
                    cases.push((hi + 1., "high"));
                }
            }
            for (value, status) in cases {
                input.value = value;
                let out = reference_guides::evaluate(&input, &profile);
                assert_eq!(
                    out["status"], status,
                    "{metric} / {} at {value}: {out}",
                    rule["id"]
                );
                assert_eq!(out["rule_id"], rule["id"]);
                checked += 1;
            }
        }
    }
    assert!(checked > 400, "Only {checked} values checked");
}

#[test]
fn fitness_rule_is_selected_again_for_each_measurement_age_and_sex() {
    let mut r = request("vo2_max", 40., "ml/kg*min");
    let mut p = Profile {
        birth_date: Some("1996-06-15".into()),
        reference_sex: Some("male".into()),
    };
    let timestamp = |date: &str| {
        chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp()
    };
    r.measured_at = timestamp("2026-06-14");
    let before = reference_guides::evaluate(&r, &p);
    assert_eq!(before["status"], "below_average");
    assert_eq!(before["applicable_rule"]["min_age"], 20);
    r.measured_at = timestamp("2026-06-15");
    let after = reference_guides::evaluate(&r, &p);
    assert_eq!(after["status"], "above_average");
    assert_eq!(after["applicable_rule"]["min_age"], 30);
    p.reference_sex = Some("female".into());
    assert_eq!(reference_guides::evaluate(&r, &p)["status"], "high");
    p.birth_date = Some("2010-06-15".into());
    assert_eq!(reference_guides::evaluate(&r, &p)["status"], "not_assessed");
}

#[test]
fn selected_rule_has_only_the_population_and_bounds_actually_used() {
    let result = reference_guides::evaluate(&request("dhea_sulfate", 600., "µg/dL"), &male());
    assert_eq!(result["applicable_rule"]["id"], result["rule_id"]);
    assert_eq!(result["applicable_rule"]["source"], result["source"]);
    assert_eq!(
        result["applicable_rule"]["lower"].as_f64(),
        result["lower"].as_f64()
    );
    assert_eq!(
        result["applicable_rule"]["upper"].as_f64(),
        result["upper"].as_f64()
    );
}

#[test]
fn weight_bounds_recompute_from_height_and_do_not_assume_missing_height() {
    let mut r = request("body_mass", 80., "kg");
    assert_eq!(
        reference_guides::evaluate(&r, &male())["status"],
        "not_assessed"
    );
    r.context.insert("height_m".into(), "1.8".into());
    let a = reference_guides::evaluate(&r, &male());
    assert_eq!(a["status"], "in_range");
    assert!((a["upper"].as_f64().unwrap() - 81.).abs() < 0.0001);
    r.context.insert("height_m".into(), "1.6".into());
    assert_eq!(
        reference_guides::evaluate(&r, &male())["label"],
        "Obesity class 1"
    );
    r.context.insert("height_m".into(), "NaN".into());
    assert_eq!(
        reference_guides::evaluate(&r, &male())["status"],
        "not_assessed"
    );
}

#[test]
fn blood_pressure_levels_do_not_require_unrecorded_resting_or_sex_context() {
    let profile = Profile {
        birth_date: Some("1980-06-15".into()),
        reference_sex: None,
    };
    for (metric, values) in [
        (
            "blood_pressure_systolic",
            vec![
                (119.0, "in_range"),
                (120.0, "elevated"),
                (130.0, "high"),
                (140.0, "very_high"),
            ],
        ),
        (
            "blood_pressure_diastolic",
            vec![(79.0, "in_range"), (80.0, "high"), (90.0, "very_high")],
        ),
    ] {
        for (value, expected) in values {
            let assessment = reference_guides::evaluate(&request(metric, value, "mmHg"), &profile);
            assert_eq!(assessment["status"], expected);
            assert_eq!(assessment["source"], "bp");
        }
    }
    let mut chart = json!({"chart_ranges":{"reference_days":[{"source":"watch","unit":"mmHg","latest_value":135,"last_at":1767225600}]}});
    reference_guides::annotate(&mut chart, "blood_pressure_systolic", &profile);
    assert_eq!(
        chart["chart_ranges"]["reference_days"][0]["reference_assessment"]["status"],
        "high"
    );
    assert!(
        !chart["reference_guidance"]["citations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn conditional_ranges_are_visible_without_inventing_measurement_context() {
    let r = request("oxygen_saturation", 98., "%");
    let conditional = reference_guides::evaluate(&r, &male());
    assert_eq!(conditional["status"], "not_assessed");
    assert_eq!(conditional["label"], "Context required");
    assert_eq!(conditional["applicable_rule"]["lower"], 95);
    assert_eq!(conditional["applicable_rule"]["upper"], 100);
    let mut r = r;
    r.context.insert("altitude".into(), "sea_level".into());
    r.context.insert("oxygen_therapy".into(), "no".into());
    assert_eq!(
        reference_guides::evaluate(&r, &male())["status"],
        "in_range"
    );
    let profile = Profile {
        birth_date: male().birth_date,
        reference_sex: None,
    };
    assert_eq!(
        reference_guides::evaluate(&request("resting_heart_rate", 65., "count/min"), &profile)["status"],
        "in_range"
    );
}

#[test]
fn unreported_fhir_units_keep_the_observations_own_printed_interval() {
    let mut chart = json!({"points":[{"source_key":"hospital","source_name":"Hospital Laboratory","timestamp":1767225600,"value":12,"original":{"clinical_observation":{"valueQuantity":{"value":12},"referenceRange":[{"low":{"value":5},"high":{"value":17}}]}}}],"chart_ranges":{"reference_days":[{"source":"hospital","last_at":1767225600,"latest_value":12,"unit":"Not reported"}]}});
    reference_guides::annotate(&mut chart, "unknown-assay", &male());
    assert_eq!(
        chart["chart_ranges"]["reference_days"][0]["reference_assessment"]["status"],
        "in_range"
    );
    assert_eq!(
        chart["chart_ranges"]["reference_days"][0]["reference_assessment"]["lower"],
        5.
    );
    assert_eq!(
        chart["chart_ranges"]["reference_days"][0]["reference_assessment"]["reference_origin"]["name"],
        "Hospital Laboratory"
    );
    chart["chart_ranges"]["reference_days"][0]["unit"] = json!("mg/dL");
    reference_guides::annotate(&mut chart, "unknown-assay", &male());
    assert_eq!(
        chart["chart_ranges"]["reference_days"][0]["reference_assessment"]["status"],
        "not_assessed"
    );
}
