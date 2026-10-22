use longlivehealth::chart::ranges;

#[test]
fn dense_ranges_preserve_extremes_singletons_sources_and_units() {
    let result = ranges(
        [
            ("watch".into(), "bpm".into(), 10.0, 60.0),
            ("watch".into(), "bpm".into(), 15.0, 110.0),
            ("watch".into(), "bpm".into(), 15.0, 110.0),
            ("strap".into(), "bpm".into(), 15.0, 80.0),
            ("watch".into(), "other".into(), 15.0, 2.0),
        ],
        chrono_tz::UTC,
    );
    for period in ["H", "D", "W", "M", "6M", "Y"] {
        let rows = result[period].as_array().unwrap();
        assert_eq!(rows.len(), 3);
        let watch = rows
            .iter()
            .find(|p| p["source"] == "watch" && p["unit"] == "bpm")
            .unwrap();
        assert_eq!(watch["min"], 60.0);
        assert_eq!(watch["max"], 110.0);
        assert_eq!(watch["count"], 2);
        let single = rows.iter().find(|p| p["source"] == "strap").unwrap();
        assert_eq!(single["min"], single["max"]);
        assert_eq!(single["count"], 1);
        assert_eq!(single["start_at"], watch["start_at"]);
        assert_eq!(single["end_at"], watch["end_at"]);
    }
}

#[test]
fn hour_uses_minutes_day_uses_hours_and_headers_preserve_latest() {
    let result = ranges(
        [
            ("watch".into(), "bpm".into(), 10.0, 60.0),
            ("watch".into(), "bpm".into(), 55.0, 110.0),
            ("watch".into(), "bpm".into(), 65.0, 75.0),
            ("watch".into(), "bpm".into(), 3600.0, 80.0),
        ],
        chrono_tz::UTC,
    );
    assert_eq!(result["H"].as_array().unwrap().len(), 3);
    assert_eq!(result["D"].as_array().unwrap().len(), 2);
    assert_eq!(result["H"][0]["start_at"], 0);
    assert_eq!(result["H"][0]["end_at"], 60);
    assert_eq!(result["D"][0]["at"], 1800.0);
    assert_eq!(result["D"][0]["count"], 3);
    assert_eq!(result["summaries"]["D"][0]["min"], 60.0);
    assert_eq!(result["summaries"]["D"][0]["max"], 110.0);
    assert_eq!(result["summaries"]["D"][0]["latest_value"], 80.0);
}

#[test]
fn calendar_buckets_respect_local_dates_dst_and_month_lengths() {
    let at = chrono::DateTime::parse_from_rfc3339("2026-03-08T16:00:00Z")
        .unwrap()
        .timestamp() as f64;
    let result = ranges(
        [("watch".into(), "bpm".into(), at, 80.0)],
        chrono_tz::America::New_York,
    );
    let day = &result["summaries"]["D"][0];
    assert_eq!(
        day["end_at"].as_i64().unwrap() - day["start_at"].as_i64().unwrap(),
        23 * 3600
    );
    let month = &result["Y"][0];
    assert_eq!(
        month["end_at"].as_i64().unwrap() - month["start_at"].as_i64().unwrap(),
        31 * 86400 - 3600
    );
    assert_eq!(result["W"][0]["start_at"], day["start_at"]);
}
