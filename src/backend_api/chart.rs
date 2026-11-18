//! Calendar-aligned display ranges; originals and source identities remain unchanged.
use chrono::{Datelike, Duration, TimeZone, Timelike};
use chrono_tz::Tz;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub const ALL_PERIOD: &str = "All";
const CALENDAR_PERIODS: [&str; 6] = ["H", "D", "W", "M", "6M", "Y"];

#[derive(Clone, Copy)]
pub(crate) struct AllHistoryBuckets {
    origin: i64,
    width: i64,
}
impl AllHistoryBuckets {
    /// Equal-duration slots, at most 48 over the archive's half-open span.
    pub(crate) fn new(start: i64, end: i64) -> Self {
        let elapsed = (end - start).max(1);
        Self {
            origin: start,
            width: (elapsed / 48 + i64::from(elapsed % 48 != 0)).max(1),
        }
    }

    fn interval(self, at: f64) -> (i64, i64) {
        let start =
            self.origin + (at.floor() as i64 - self.origin).div_euclid(self.width) * self.width;
        (start, start + self.width)
    }
}

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
    latest_bounds: Option<(f64, f64)>,
    interval_count: usize,
    count: usize,
    mean: f64,
}
impl Range {
    fn add(&mut self, at: f64, value: f64) {
        self.add_bounds(at, value, value, false);
    }

    fn add_bounds(&mut self, at: f64, lower: f64, upper: f64, is_interval: bool) {
        self.first = self.first.min(at);
        if at >= self.last {
            self.last = at;
            self.latest = lower;
            self.latest_bounds = is_interval.then_some((lower, upper));
        }
        self.min = self.min.min(lower);
        self.max = self.max.max(upper);
        self.count += 1;
        self.interval_count += usize::from(is_interval);
        if !is_interval {
            self.mean += (lower - self.mean) / (self.count - self.interval_count) as f64;
        }
    }
    fn json(&self, source: &str, unit: &str, start: i64) -> Value {
        let mut row = json!({"source":source,"unit":unit,"at":(start as f64+self.end as f64)/2.0,
            "start_at":start,"end_at":self.end,"first_at":self.first,"last_at":self.last,
            "min":self.min,"max":self.max,"latest_value":self.latest_bounds.is_none().then_some(self.latest),
            "mean":(self.interval_count == 0).then_some(self.mean),"count":self.count});
        if let Some((lower, upper)) = self.latest_bounds {
            row["latest_min"] = json!(lower);
            row["latest_max"] = json!(upper);
        }
        row
    }
}

pub fn ranges(
    samples: impl IntoIterator<Item = (String, String, f64, f64), IntoIter: Clone>,
    timezone: Tz,
) -> Value {
    bounded_ranges(
        samples
            .into_iter()
            .map(|(source, unit, at, value)| (source, unit, at, value, None)),
        timezone,
        false,
    )
}

pub(crate) fn bounded_ranges(
    samples: impl Iterator<Item = (String, String, f64, f64, Option<f64>)> + Clone,
    timezone: Tz,
    binary: bool,
) -> Value {
    let samples = samples.filter(|(_, _, at, lower, upper)| {
        let upper = upper.unwrap_or(*lower);
        at.is_finite()
            && chrono::DateTime::from_timestamp(at.floor() as i64, 0).is_some()
            && lower.is_finite()
            && upper.is_finite()
            && *lower <= upper
    });
    let (first, last) =
        samples
            .clone()
            .fold((i64::MAX, i64::MIN), |(first, last), (_, _, at, _, _)| {
                let at = at.floor() as i64;
                (first.min(at), last.max(at))
            });
    let all = (first <= last).then(|| AllHistoryBuckets::new(first, last + 1));
    let mut groups = BTreeMap::new();
    let mut summaries = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for (source, unit, at, lower, upper) in samples {
        let is_interval = upper.is_some();
        let upper = upper.unwrap_or(lower);
        if !seen.insert((
            source.clone(),
            unit.clone(),
            at.to_bits(),
            lower.to_bits(),
            upper.to_bits(),
            is_interval,
        )) {
            continue;
        }
        for period in CALENDAR_PERIODS.into_iter().chain([ALL_PERIOD]) {
            for (window, target) in [(false, &mut groups), (true, &mut summaries)] {
                let bounds = if period == ALL_PERIOD {
                    all.filter(|_| !window).map(|all| all.interval(at))
                } else {
                    interval(at, period, timezone, window)
                };
                let Some((start, end)) = bounds else { continue };
                let entry = target
                    .entry((
                        period,
                        source.clone(),
                        unit.clone(),
                        start,
                        binary.then(|| lower.to_bits()),
                    ))
                    .or_insert(Range {
                        end,
                        first: at,
                        last: at,
                        min: lower,
                        max: upper,
                        latest: lower,
                        latest_bounds: None,
                        interval_count: 0,
                        count: 0,
                        mean: 0.0,
                    });
                entry.add_bounds(at, lower, upper, is_interval);
            }
        }
    }
    let mut output = json!({"H":[],"D":[],"W":[],"M":[],"6M":[],"Y":[],"All":[],"summaries":{}});
    for ((period, source, unit, start, _), entry) in groups {
        output[period]
            .as_array_mut()
            .unwrap()
            .push(entry.json(&source, &unit, start));
    }
    for period in CALENDAR_PERIODS {
        output["summaries"][period] = json!([]);
    }
    // Window headers combine extrema only, never sum or average independent sources.
    let mut headers: BTreeMap<(&str, String, i64), Range> = BTreeMap::new();
    for ((period, _, unit, start, _), entry) in summaries {
        let merged = headers
            .entry((period, unit, start))
            .or_insert(entry.clone());
        merged.min = merged.min.min(entry.min);
        merged.max = merged.max.max(entry.max);
        merged.interval_count = merged.interval_count.max(entry.interval_count);
        if entry.last > merged.last {
            merged.last = entry.last;
            merged.latest = entry.latest;
            merged.latest_bounds = entry.latest_bounds;
        }
    }
    for ((period, unit, start), entry) in headers {
        output["summaries"][period]
            .as_array_mut()
            .unwrap()
            .push(entry.json("", &unit, start));
    }
    output["reference_days"] = output["W"].clone();
    output
}

/// One selected calendar period: bounded by buckets and sources, not sample count.
pub struct PeriodRanges {
    period: String,
    timezone: Tz,
    groups: BTreeMap<(String, String, i64), Range>,
    all: Option<AllHistoryBuckets>,
}
impl PeriodRanges {
    pub(crate) fn new(period: &str, timezone: Tz, span: (i64, i64)) -> Option<Self> {
        (CALENDAR_PERIODS.contains(&period) || period == ALL_PERIOD).then(|| Self {
            period: period.into(),
            timezone,
            groups: BTreeMap::new(),
            all: (period == ALL_PERIOD).then(|| AllHistoryBuckets::new(span.0, span.1)),
        })
    }
    pub fn add(&mut self, source: String, unit: String, at: f64, value: f64) {
        if !at.is_finite() || !value.is_finite() {
            return;
        }
        let bounds = match self.all {
            Some(all) => Some(all.interval(at)),
            None => interval(at, &self.period, self.timezone, false),
        };
        if let Some((start, end)) = bounds {
            self.groups
                .entry((source, unit, start))
                .or_insert(Range {
                    end,
                    first: at,
                    last: at,
                    min: value,
                    max: value,
                    latest: value,
                    latest_bounds: None,
                    interval_count: 0,
                    count: 0,
                    mean: 0.0,
                })
                .add(at, value);
        }
    }
    pub fn finish(self) -> Value {
        let mut result = json!({"H":[],"D":[],"W":[],"M":[],"6M":[],"Y":[],"All":[]});
        result[&self.period] = json!(
            self.groups
                .into_iter()
                .map(|((source, unit, start), range)| range.json(&source, &unit, start))
                .collect::<Vec<_>>()
        );
        result
    }
}
