//! Calendar-aligned display ranges; originals and source identities remain unchanged.
use chrono::{Datelike, Duration, TimeZone, Timelike};
use chrono_tz::Tz;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn interval(at: f64, period: &str, timezone: Tz, window: bool) -> Option<(i64, i64)> {
    let date = chrono::DateTime::from_timestamp(at.floor() as i64, 0)?.with_timezone(&timezone);
    if period == "H" || (period == "D" && !window) {
        let width = if period == "H" && !window { 60 } else { 3600 };
        let start = date.timestamp()
            - i64::from(date.second())
            - if width == 3600 {
                i64::from(date.minute()) * 60
            } else {
                0
            };
        return Some((start, start + width));
    }
    let day = date.date_naive();
    let (start, end) = match (period, window) {
        ("D", true) | ("W" | "M", false) => (day, day.succ_opt()?),
        ("W", true) | ("6M", false) => {
            // Sunday matches the chart's explicit Gregorian calendar.
            let start = day.checked_sub_signed(Duration::days(i64::from(
                day.weekday().num_days_from_sunday(),
            )))?;
            (start, start.checked_add_signed(Duration::days(7))?)
        }
        ("Y", false) | ("M", true) => {
            let start = day.with_day(1)?;
            (start, start.checked_add_months(chrono::Months::new(1))?)
        }
        ("6M", true) => {
            let start = day
                .with_day(1)?
                .with_month(if day.month() <= 6 { 1 } else { 7 })?;
            (start, start.checked_add_months(chrono::Months::new(6))?)
        }
        ("Y", true) => {
            let start = day.with_day(1)?.with_month(1)?;
            (start, start.checked_add_months(chrono::Months::new(12))?)
        }
        _ => return None,
    };
    Some((
        timezone
            .from_local_datetime(&start.and_hms_opt(0, 0, 0)?)
            .earliest()?
            .timestamp(),
        timezone
            .from_local_datetime(&end.and_hms_opt(0, 0, 0)?)
            .earliest()?
            .timestamp(),
    ))
}

#[derive(Clone)]
struct Range {
    end: i64,
    first: f64,
    last: f64,
    min: f64,
    max: f64,
    latest: f64,
    count: usize,
}
impl Range {
    fn add(&mut self, at: f64, value: f64) {
        self.first = self.first.min(at);
        if at >= self.last {
            self.last = at;
            self.latest = value;
        }
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.count += 1;
    }
    fn json(&self, source: &str, unit: &str, start: i64) -> Value {
        json!({"source":source,"unit":unit,"at":(start as f64+self.end as f64)/2.0,
            "start_at":start,"end_at":self.end,"first_at":self.first,"last_at":self.last,
            "min":self.min,"max":self.max,"latest_value":self.latest,"count":self.count})
    }
}

pub fn ranges(
    samples: impl IntoIterator<Item = (String, String, f64, f64)>,
    timezone: Tz,
) -> Value {
    let mut groups = BTreeMap::new();
    let mut summaries = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for (source, unit, at, value) in samples {
        if !at.is_finite()
            || !value.is_finite()
            || !seen.insert((source.clone(), unit.clone(), at.to_bits(), value.to_bits()))
        {
            continue;
        }
        for period in ["H", "D", "W", "M", "6M", "Y"] {
            for (window, target) in [(false, &mut groups), (true, &mut summaries)] {
                let Some((start, end)) = interval(at, period, timezone, window) else {
                    continue;
                };
                let entry = target
                    .entry((period, source.clone(), unit.clone(), start))
                    .or_insert(Range {
                        end,
                        first: at,
                        last: at,
                        min: value,
                        max: value,
                        latest: value,
                        count: 0,
                    });
                entry.add(at, value);
            }
        }
    }
    let mut output = json!({"H":[],"D":[],"W":[],"M":[],"6M":[],"Y":[],"summaries":{}});
    for ((period, source, unit, start), entry) in groups {
        output[period]
            .as_array_mut()
            .unwrap()
            .push(entry.json(&source, &unit, start));
    }
    for period in ["H", "D", "W", "M", "6M", "Y"] {
        output["summaries"][period] = json!([]);
    }
    // Window headers combine extrema only, never sum or average independent sources.
    let mut headers: BTreeMap<(&str, String, i64), Range> = BTreeMap::new();
    for ((period, _, unit, start), entry) in summaries {
        let merged = headers
            .entry((period, unit, start))
            .or_insert(entry.clone());
        merged.min = merged.min.min(entry.min);
        merged.max = merged.max.max(entry.max);
        if entry.last > merged.last {
            merged.last = entry.last;
            merged.latest = entry.latest;
        }
    }
    for ((period, unit, start), entry) in headers {
        output["summaries"][period]
            .as_array_mut()
            .unwrap()
            .push(entry.json("", &unit, start));
    }
    output
}
