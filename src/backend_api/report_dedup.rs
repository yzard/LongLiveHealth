use crate::{laboratory, laboratory_names, reports::ObservationPayload};
use rust_decimal::Decimal;

#[derive(Clone, Eq, Hash, PartialEq)]
pub(crate) struct ResultKey {
    name: String,
    matched_metric: String,
    sampled_at: String,
    unit: String,
    value: Decimal,
}

pub(crate) fn result_key(
    payload: &ObservationPayload,
    metric_id: Option<&str>,
    document_date: Option<&str>,
) -> Option<ResultKey> {
    let value = Decimal::from_str_exact(payload.raw_result.trim())
        .ok()?
        .normalize();
    let date = payload
        .sampled_at
        .as_deref()
        .filter(|date| !date.trim().is_empty())
        .or(document_date)?;
    let sampled_at = if date.len() == 10 {
        chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .ok()?
            .to_string()
    } else {
        chrono::DateTime::parse_from_rfc3339(date)
            .ok()?
            .to_utc()
            .to_rfc3339()
    };
    let (name, code) = laboratory::split_test_code(&payload.raw_name);
    let matched_metric = match (metric_id, code, payload.ocr_metric_candidate.as_deref()) {
        (Some(id), _, _) => id.to_owned(),
        (None, Some(code), suggested) => format!("loinc:{code}:{}", suggested.unwrap_or("")),
        (None, None, Some(suggested)) => format!("suggested:{suggested}"),
        (None, None, None) => return None,
    };
    Some(ResultKey {
        name: laboratory_names::key(name),
        matched_metric,
        sampled_at,
        unit: payload
            .raw_unit
            .as_deref()
            .map(laboratory::custom_unit)
            .unwrap_or_default()
            .into(),
        value,
    })
}
