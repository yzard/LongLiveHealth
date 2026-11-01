//! Explicit health measurements only. Archive IDs, status codes and metadata are never axes.
use serde_json::{Value, json};

pub fn supports(kind: &str) -> bool {
    [
        "activity_summary",
        "state_of_mind",
        "vision_prescription",
        "heartbeat_series",
        "workout",
        "medication_dose_event",
    ]
    .contains(&kind)
        || crate::apple_health::definition(kind)
            .is_some_and(|d| d.family == "category" && kind != "sleep")
}

pub fn project(kind: &str, payload: &Value, start: f64, end: f64) -> Value {
    if !supports(kind) {
        return json!([]);
    }
    let mut samples = Vec::new();
    let decoded = if ["state_of_mind", "vision_prescription"].contains(&kind) {
        payload["raw_archive"]["data"]
            .as_str()
            .and_then(crate::apple_archive::decode)
            .unwrap_or(Value::Null)
    } else {
        Value::Null
    };
    let mut add = |id: &str, title: &str, unit: &str, value: Option<f64>| {
        if let Some(value) = value.filter(|v| v.is_finite()) {
            samples.push(json!({"id":id,"title":title,"unit":unit,"value":value}));
        }
    };
    match kind {
        "workout" => {
            add(
                "workout_duration",
                "Workout duration",
                "min",
                payload["duration_seconds"].as_f64().map(|v| v / 60.0),
            );
            add(
                "workout_energy",
                "Workout energy",
                "kcal",
                payload["energy_kcal"].as_f64(),
            );
            add(
                "workout_distance",
                "Workout distance",
                "m",
                payload["distance_m"].as_f64(),
            );
        }
        "medication_dose_event" => add("event_count", "Dose log entries", "count", Some(1.0)),
        "state_of_mind" => add(
            "valence",
            "Mood valence",
            "score",
            payload["valence"]
                .as_f64()
                .or_else(|| decoded["Valence"].as_f64())
                .filter(|v| (-1.0..=1.0).contains(v)),
        ),
        "activity_summary" => {
            // These portable fields are exported with explicit HealthKit units by the client.
            // Archived untyped floats have no documented unit contract and are not guessed.
            for (key, title, unit) in [
                ("active_energy_kcal", "Active energy", "kcal"),
                ("exercise_minutes", "Exercise time", "min"),
                ("stand_hours", "Stand hours", "hr"),
                ("move_minutes", "Move time", "min"),
            ] {
                add(key, title, unit, payload[key].as_f64());
            }
        }
        "vision_prescription" => {
            for (eye, label) in [("LeftEye", "Left eye"), ("RightEye", "Right eye")] {
                for (key, title) in [
                    ("Sphere", "sphere"),
                    ("Cyclinder", "cylinder"),
                    ("AddPower", "addition"),
                    ("Axis", "axis"),
                ] {
                    let q = &decoded[eye][key];
                    if let Some(unit) = q["UnitKey"]["HKUnitStringKey"].as_str() {
                        add(
                            &format!("{eye}_{key}"),
                            &format!("{label} {title}"),
                            unit,
                            q["ValueKey"].as_f64(),
                        );
                    }
                }
            }
        }
        "heartbeat_series" => {
            let mut last = None;
            let mut total = 0.0;
            let mut count = 0;
            for beat in payload["series"].as_array().into_iter().flatten() {
                if let Some(at) = beat["time_since_start"].as_f64() {
                    if let Some(previous) = last
                        && beat["preceded_by_gap"] != true
                        && at > previous
                    {
                        total += at - previous;
                        count += 1;
                    }
                    last = Some(at);
                }
            }
            if count > 0 {
                add(
                    "mean_beat_interval",
                    "Mean beat interval",
                    "ms",
                    Some(total * 1000.0 / f64::from(count)),
                );
            }
        }
        "mindful_session" => add(
            "duration_minutes",
            "Session duration",
            "min",
            (end >= start).then_some((end - start) / 60.0),
        ),
        _ if crate::apple_health::definition(kind)
            .is_some_and(|d| d.family == "category" && kind != "sleep") =>
        {
            add("event_count", "Recorded events", "count", Some(1.0));
        }
        _ => {}
    }
    json!(samples)
}

pub fn timestamp(kind: &str, payload: &Value) -> Option<i64> {
    use chrono::TimeZone;
    if kind != "activity_summary" {
        return None;
    }
    let day = chrono::NaiveDate::parse_from_str(payload["date"].as_str()?, "%Y-%m-%d").ok()?;
    let tz: chrono_tz::Tz = payload["timezone"].as_str()?.parse().ok()?;
    Some(
        tz.from_local_datetime(&day.and_hms_opt(0, 0, 0)?)
            .earliest()?
            .timestamp(),
    )
}
