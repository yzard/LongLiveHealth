//! Presentation-ready evidence. Scores are never reverse-engineered or invented.
use serde_json::{Value, json};

fn rows(value: &Value) -> impl Iterator<Item = &Value> {
    value.as_array().into_iter().flatten()
}
fn last(card: &Value) -> Option<(i64, f64)> {
    super::points(card, None).into_iter().max_by_key(|p| p.0)
}
fn evidence(cards: &[Value], kinds: &[&str], as_of: i64) -> Vec<Value> {
    let mut result: Vec<_> = cards
        .iter()
        .filter(|c| kinds.contains(&c["metric_id"].as_str().unwrap_or("")))
        .filter_map(|c| {
            let (at, value) = last(c)?;
            if at > as_of {
                return None;
            }
            let mut item = c.clone();
            item["latest"] = json!(value);
            item["latest_at"] = json!(at);
            Some(item)
        })
        .collect();
    result.sort_by(|a, b| {
        b["latest_at"]
            .as_i64()
            .cmp(&a["latest_at"].as_i64())
            .then_with(|| a["id"].as_str().cmp(&b["id"].as_str()))
    });
    result.sort_by_key(|item| {
        kinds
            .iter()
            .position(|kind| Some(*kind) == item["metric_id"].as_str())
            .unwrap_or(kinds.len())
    });
    result
}
fn panel(id: &str, title: &str, symbol: &str, summary: &str, evidence: Vec<Value>) -> Value {
    json!({"id":id,"title":title,"symbol":symbol,"summary":summary,"evidence":evidence,
        "method":"Charts retain separate sources and original units. The preview uses the most recent available source; details show all sources. Dates are shown because an older observation is not today's health status.","references":[]})
}
pub fn build(
    cards: &[Value],
    cardiac: &Value,
    training: &Value,
    sleep: &Value,
    goal: Option<f64>,
    as_of: i64,
) -> Value {
    let mut sleep_panel = panel(
        "sleep",
        "Sleep",
        "bed.double.fill",
        "Your recorded sleep duration and stages.",
        evidence(cards, &["sleep"], as_of),
    );
    let latest_session = rows(&sleep["sources"])
        .flat_map(|source| rows(&source["sessions"]).map(move |session| (source, session)))
        .filter(|(_, s)| {
            s["state"] == "ready" && s["end_at"].as_i64().is_some_and(|at| at <= as_of)
        })
        .max_by_key(|(_, s)| {
            (
                s["wake_date"].as_str(),
                s["asleep_seconds"].as_i64(),
                s["end_at"].as_i64(),
            )
        });
    if let Some((source, session)) = latest_session {
        sleep_panel["session"] = session.clone();
        sleep_panel["source"] = source["source"].clone();
    }
    sleep_panel["goal_minutes"] = json!(goal);
    sleep_panel["method"] = json!(
        "The longest complete recorded session on the latest recorded wake date is shown, without combining devices. This is not a validated main-sleep classifier. The ring shows duration relative to your own sleep goal, not an Apple Sleep Score. Stages come from the original records; gaps are not assumed awake. Apple's published score weights are duration 50, bedtime consistency 30 and interruptions 20; complete point curves are not available in the cited documentation."
    );
    sleep_panel["references"] = json!([{"title":"Apple — Sleep Score and sleep stages","url":"https://support.apple.com/en-ie/108906"}]);
    let mut readiness = panel(
        "readiness",
        "Readiness",
        "sparkles",
        training["status"]
            .as_str()
            .unwrap_or("More comparable data needed"),
        rows(&training["signals"]).cloned().collect(),
    );
    readiness["method"] = json!(format!(
        "{} Apple Readiness uses activity, vitals and sleep, but its full numerical formula is not published in the linked documentation. This review shows contributing evidence, not an Apple-equivalent score.",
        training["method"].as_str().unwrap_or("")
    ));
    readiness["references"] = training["references"].clone();
    readiness["references"].as_array_mut().expect("references array").push(json!({"title":"Apple — Readiness factors and classifications","url":"https://support.apple.com/en-ie/128112"}));
    let mut vitals = panel(
        "vitals",
        "Vitals",
        "waveform.path.ecg",
        "Recent daily readings against your own recorded baseline.",
        evidence(
            cards,
            &[
                "resting_heart_rate",
                "respiratory_rate",
                "oxygen_saturation",
                "HKQuantityTypeIdentifierAppleSleepingWristTemperature",
                crate::recovery_hrv::KIND,
            ],
            as_of,
        ),
    );
    for item in vitals["evidence"].as_array_mut().expect("evidence array") {
        let latest_at = item["latest_at"].as_i64().expect("latest timestamp");
        let old: Vec<_> = super::points(item, None)
            .into_iter()
            .filter(|(at, _)| *at < latest_at && *at >= latest_at - 28 * 86400)
            .map(|(_, v)| v)
            .collect();
        if old.len() >= 14 {
            item["baseline_low"] = json!(old.iter().copied().reduce(f64::min));
            item["baseline_high"] = json!(old.iter().copied().reduce(f64::max));
        }
        item["baseline_days"] = json!(old.len());
    }
    vitals["method"] = json!(
        "Each source is kept separate. The band is the observed minimum–maximum of that source's daily values in the 28 days preceding its latest day, with at least 14 observed days. It is a descriptive baseline, not a clinical normal range or Apple's Typical classification. Daily means can include daytime readings; these are not labeled overnight vitals. Missing days are not filled."
    );
    let heart = panel(
        "heart",
        "Heart",
        "heart.fill",
        "Heart rate and variability, with each measurement protocol kept separate.",
        evidence(
            cards,
            &[
                "heart_rate",
                "resting_heart_rate",
                "hrv_sdnn",
                "hrv_rmssd",
                crate::recovery_hrv::KIND,
            ],
            as_of,
        ),
    );
    let fitness = panel(
        "fitness",
        "Fitness",
        "figure.walk",
        "Recorded activity and cardio fitness over time.",
        evidence(cards, &["active_energy", "workout", "vo2_max"], as_of),
    );
    let mut cardiovascular = panel(
        "cardiovascular",
        "Cardiovascular Review",
        "heart.fill",
        "A combined review of blood pressure, blood work and your risk context.",
        rows(&cardiac["cards"]).cloned().collect(),
    );
    cardiovascular["method"] = cardiac["method"].clone();
    cardiovascular["references"] = cardiac["references"].clone();
    json!([
        sleep_panel,
        readiness,
        vitals,
        heart,
        fitness,
        cardiovascular
    ])
}
