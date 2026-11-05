//! Display decisions for changes; thresholds are product filters, not diagnostic cutoffs.
use crate::reference_guides::{self, EvaluationRequest, Profile};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn number(v: &Value) -> Option<f64> {
    v.as_f64()
        .or_else(|| v.as_str()?.parse().ok())
        .filter(|v| v.is_finite())
}
fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let n = values.len();
    if n.is_multiple_of(2) {
        values[n / 2 - 1] / 2. + values[n / 2] / 2.
    } else {
        values[n / 2]
    }
}
fn target(rule: &Value) -> Option<(Option<f64>, Option<f64>)> {
    if let Some(bands) = rule["bands"].as_array().filter(|b| !b.is_empty()) {
        let normal = bands.iter().find(|b| b["status"] == "in_range")?;
        Some((number(&normal["lower"]), number(&normal["upper"])))
    } else {
        let bounds = (number(&rule["lower"]), number(&rule["upper"]));
        (bounds.0.is_some() || bounds.1.is_some()).then_some(bounds)
    }
}
fn distance(value: f64, bounds: (Option<f64>, Option<f64>)) -> f64 {
    bounds.0.map_or(0., |lo| (lo - value).max(0.)) + bounds.1.map_or(0., |hi| (value - hi).max(0.))
}
fn assessment(metric: &str, value: f64, unit: &str, at: i64, profile: &Profile) -> Value {
    reference_guides::evaluate(
        &EvaluationRequest {
            metric_id: metric.into(),
            value,
            unit: unit.into(),
            measured_at: at,
            context: BTreeMap::new(),
        },
        profile,
    )
}

/// Classify components independently; any worsening component takes priority over improving components.
pub(super) fn classify(
    card: &Value,
    profile: &Profile,
    sleep_minutes: Option<f64>,
    now: i64,
) -> Option<Value> {
    if card["state"] != "ready" {
        return None;
    }
    let metric = card["metric_id"].as_str()?;
    let daily = card["category"] == "daily";
    let last = card["last_at"].as_i64()?;
    // Historical retired-device series are not current changes. Lab monitoring is less frequent.
    let freshness = if card["category"] == "bloodWork" {
        366
    } else {
        90
    };
    if now - last > freshness * 86400 {
        return None;
    }
    let mut directions = Vec::new();
    let mut comparisons = Vec::new();
    for line in card["series"].as_array()? {
        let points = line["points"].as_array()?;
        let mut older = Vec::new();
        let mut recent = Vec::new();
        let mut earlier_points = Vec::new();
        let mut recent_points = Vec::new();
        let split = if daily {
            card["recent_start"].as_i64()?
        } else if card["category"] != "bloodWork" && card["aggregation"] == "Daily averages" {
            last - 14 * 86400
        } else {
            last - 90 * 86400
        };
        let use_windows = daily
            || points
                .iter()
                .filter(|p| p["at"].as_i64().is_some_and(|at| at >= split))
                .count()
                >= 2
                && points
                    .iter()
                    .filter(|p| p["at"].as_i64().is_some_and(|at| at < split))
                    .count()
                    >= 2;
        if use_windows {
            for p in points {
                let at = p["at"].as_i64()?;
                let value = number(&p["value"])?;
                if at >= split {
                    recent.push(value);
                    recent_points.push(p.clone());
                } else {
                    older.push(value);
                    earlier_points.push(p.clone());
                }
            }
            if daily && (recent.len() < 4 || older.len() < 14) {
                return None;
            }
        } else {
            // Sparse blood tests: compare the last two collection dates, not arbitrary endpoints years apart.
            let final_date = points.last()?["date"].as_str()?;
            let previous_date = points
                .iter()
                .rev()
                .find(|p| p["date"].as_str().is_some_and(|d| d != final_date))?["date"]
                .as_str()?;
            for p in points {
                if p["date"] == final_date {
                    recent.push(number(&p["value"])?);
                    recent_points.push(p.clone());
                }
                if p["date"] == previous_date {
                    older.push(number(&p["value"])?);
                    earlier_points.push(p.clone());
                }
            }
        }
        if older.is_empty() || recent.is_empty() {
            return None;
        }
        let before = median(&mut older);
        let after = median(&mut recent);
        let kind = if metric == "HKCorrelationTypeIdentifierBloodPressure" {
            if line["title"] == "Systolic" {
                "blood_pressure_systolic"
            } else {
                "blood_pressure_diastolic"
            }
        } else {
            metric
        };
        let unit = card["unit"].as_str()?;
        let guide = assessment(kind, after, unit, now, profile);
        let mut basis = "Applicable reference interval";
        let change;
        if kind == "sleep" {
            let goal = sleep_minutes? * 60.;
            if unit != "seconds" {
                return None;
            }
            // Longer sleep beyond the goal is not automatically called better or worse.
            change = (goal - after).max(0.) - (goal - before).max(0.);
            basis = "Distance below your saved sleep goal";
            if change.abs() < 30. * 60. {
                continue;
            }
        } else if kind == "vo2_max" {
            if guide["kind"] != "fitness_percentile" || guide["status"] == "not_assessed" {
                return None;
            }
            change = before - after;
            basis = "Cardio fitness estimate; age and biological sex guide";
            if change.abs() < before.abs().max(1.) * 0.05 {
                continue;
            }
        } else {
            let printed = card["reference_interval"]
                .as_object()
                .map(|_| &card["reference_interval"]);
            let rule = if let Some(printed) = printed {
                basis = "Original report reference interval";
                printed
            } else {
                if guide["status"] == "not_assessed" {
                    return None;
                }
                &guide["applicable_rule"]
            };
            let bounds = target(rule)?;
            // Elevated-pressure guides don't assess hypotension; don't claim a low reading is an improvement.
            if metric == "HKCorrelationTypeIdentifierBloodPressure"
                && (before < if kind.ends_with("systolic") { 90. } else { 60. }
                    || after < if kind.ends_with("systolic") { 90. } else { 60. })
            {
                return None;
            }
            change = distance(after, bounds) - distance(before, bounds);
            let scale = bounds
                .0
                .zip(bounds.1)
                .map_or(before.abs().max(1.), |(lo, hi)| (hi - lo).abs().max(0.001));
            if change.abs() < scale * 0.05 {
                continue;
            }
        }
        directions.push(if change > 0. {
            "worsening"
        } else {
            "improving"
        });
        comparisons.push(json!({"title":line["title"],"before":before,"after":after,"basis":basis,"direction":if change > 0. { "worsening" } else { "improving" },"earlier_points":earlier_points,"recent_points":recent_points,"citations":reference_guides::guide(kind)["citations"]}));
    }
    directions.first()?;
    let direction = if directions.contains(&"worsening") {
        "worsening"
    } else {
        "improving"
    };
    Some(
        json!({"direction":direction,"comparisons":comparisons,"comparison":"Recent 14 observed calendar days for daily device estimates, or recent 90 days for monthly histories, versus earlier recorded history when both have at least two points; otherwise the last two collection dates. Daily metrics compare 7 completed days with the preceding 28.","filter_note":"Display filters: 5% of the reference interval width (or previous magnitude for a one-sided guide), 5% for VO₂ Max, and 30 minutes of sleep-goal shortfall. These filters reduce small fluctuations; they are not validated clinical-change thresholds."}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn card(metric: &str, unit: &str, before: f64, after: f64) -> Value {
        json!({"state":"ready","metric_id":metric,"category":"bloodWork","unit":unit,"last_at":1794000000,
        "series":[{"title":metric,"points":[{"at":1791000000,"date":"2026-10-03","value":before},{"at":1794000000,"date":"2026-11-07","value":after}]}]})
    }
    fn profile() -> Profile {
        Profile {
            birth_date: Some("1980-01-01".into()),
            reference_sex: Some("male".into()),
        }
    }
    #[test]
    fn intervals_distinguish_low_high_in_range_and_noise() {
        for (before, after, direction) in [
            (4., 3., "worsening"),
            (3., 4., "improving"),
            (5., 6., "worsening"),
            (6., 5., "improving"),
        ] {
            assert_eq!(
                classify(
                    &card("albumin", "g/dL", before, after),
                    &profile(),
                    None,
                    1794000000
                )
                .unwrap()["direction"],
                direction
            );
        }
        assert!(
            classify(
                &card("albumin", "g/dL", 4., 4.5),
                &profile(),
                None,
                1794000000
            )
            .is_none()
        );
        assert!(
            classify(
                &card("albumin", "g/dL", 5.5, 5.51),
                &profile(),
                None,
                1794000000
            )
            .is_none()
        );
        assert!(
            classify(
                &card("unknown", "g/dL", 5., 50.),
                &profile(),
                None,
                1794000000
            )
            .is_none()
        );
        assert!(
            classify(
                &card("albumin", "wrong", 5., 50.),
                &profile(),
                None,
                1794000000
            )
            .is_none()
        );
        assert!(
            classify(
                &card("albumin", "g/dL", 5., 50.),
                &Profile::default(),
                None,
                1794000000
            )
            .is_none()
        );
    }
    #[test]
    fn retired_sources_and_insufficient_cards_are_not_current_trends() {
        assert!(
            classify(
                &card("albumin", "g/dL", 5., 6.),
                &profile(),
                None,
                1794000000 + 367 * 86400
            )
            .is_none()
        );
        let mut c = card("albumin", "g/dL", 5., 6.);
        c["state"] = json!("insufficient_data");
        assert!(classify(&c, &profile(), None, 1794000000).is_none());
    }
    #[test]
    fn fitness_uses_its_own_direction_and_profile_while_small_changes_are_hidden() {
        assert_eq!(
            classify(
                &card("vo2_max", "ml/kg*min", 35., 40.),
                &profile(),
                None,
                1794000000
            )
            .unwrap()["direction"],
            "improving"
        );
        assert_eq!(
            classify(
                &card("vo2_max", "ml/kg*min", 40., 35.),
                &profile(),
                None,
                1794000000
            )
            .unwrap()["direction"],
            "worsening"
        );
        assert!(
            classify(
                &card("vo2_max", "ml/kg*min", 40., 40.1),
                &profile(),
                None,
                1794000000
            )
            .is_none()
        );
    }
    #[test]
    fn one_worsening_pressure_component_has_priority_and_preserves_both_directions() {
        let mut c = card(
            "HKCorrelationTypeIdentifierBloodPressure",
            "mmHg",
            130.,
            145.,
        );
        c["series"][0]["title"] = json!("Systolic");
        let mut dia = c["series"][0].clone();
        dia["title"] = json!("Diastolic");
        dia["points"][0]["value"] = json!(95.);
        dia["points"][1]["value"] = json!(80.);
        c["series"].as_array_mut().unwrap().push(dia);
        let result = classify(&c, &profile(), None, 1794000000).unwrap();
        assert_eq!(result["direction"], "worsening");
        assert_eq!(result["comparisons"][0]["direction"], "worsening");
        assert_eq!(result["comparisons"][1]["direction"], "improving");
    }
    #[test]
    fn sleep_goal_does_not_mark_eight_hours_below_a_seven_hour_goal() {
        let now = 1794000000;
        let mut c = card("sleep", "seconds", 21600., 28800.);
        c["category"] = json!("daily");
        c["recent_start"] = json!(now - 7 * 86400);
        c["series"][0]["points"] = json!((1..=35).rev().map(|d| json!({"at":now-d*86400,"date":d.to_string(),"value":if d<=7 {28800.} else {21600.}})).collect::<Vec<_>>());
        assert_eq!(
            classify(&c, &profile(), Some(420.), now).unwrap()["direction"],
            "improving"
        );
        for p in c["series"][0]["points"].as_array_mut().unwrap() {
            p["value"] = json!(28800.);
        }
        assert!(classify(&c, &profile(), Some(420.), now).is_none());
        assert!(classify(&c, &profile(), None, now).is_none());
    }
}
