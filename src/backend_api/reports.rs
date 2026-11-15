use crate::{
    authentication::now,
    database::{Database, queries},
    error::AppError,
    lab_metric_metadata::{self, Metadata, test_code},
    laboratory::CustomMetric,
    models::ListRequest,
};
use chrono::{DateTime, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::Row;
use std::collections::HashSet;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub page: i64,
    pub quote: String,
    pub bounding_box: Option<[f64; 4]>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationPayload {
    pub raw_name: String,
    pub raw_result: String,
    pub raw_unit: Option<String>,
    pub reference_range: Option<String>,
    pub report_flag: Option<String>,
    pub sampled_at: Option<String>,
    pub metric_id: Option<String>,
    pub source: Source,
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ocr_mapping_note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ocr_metric_candidate: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ocr_section_heading: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ocr_new_metric: Option<NewMetric>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ocr_normalization: Option<OcrNormalization>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OcrNormalization {
    pub result: String,
    pub unit: String,
}

impl ObservationPayload {
    pub fn validate(&self, pages: i64) -> Result<(), AppError> {
        if crate::laboratory::is_administrative_name(&self.raw_name) {
            return Err(AppError::Invalid(
                "Administrative report fields are not health measurements",
            ));
        }
        if self.raw_name.trim().is_empty()
            || self.raw_name.len() > 256
            || self.raw_result.trim().is_empty()
            || self.raw_result.len() > 4096
            || self.source.page < 1
            || self.source.page > pages
            || self.source.quote.len() > 8192
        {
            return Err(AppError::Invalid(
                "Invalid observation name, result or source page",
            ));
        }
        for field in [
            &self.raw_unit,
            &self.reference_range,
            &self.report_flag,
            &self.notes,
        ] {
            if field.as_ref().is_some_and(|text| text.len() > 4096) {
                return Err(AppError::Invalid("Observation field too long"));
            }
        }
        if self
            .ocr_mapping_note
            .as_ref()
            .is_some_and(|note| note.len() > 512)
        {
            return Err(AppError::Invalid("OCR mapping note exceeds limit"));
        }
        if self
            .ocr_section_heading
            .as_ref()
            .is_some_and(|heading| heading.len() > 160 || heading.chars().any(char::is_control))
        {
            return Err(AppError::Invalid("Invalid OCR section heading"));
        }
        if self
            .ocr_metric_candidate
            .as_ref()
            .is_some_and(|id| id.len() > 128)
            || self
                .ocr_new_metric
                .as_ref()
                .is_some_and(|metric| !metric.is_valid())
        {
            return Err(AppError::Invalid("Invalid OCR metric suggestion"));
        }
        if self
            .ocr_normalization
            .as_ref()
            .is_some_and(|hint| hint.result.len() > 128 || hint.unit.len() > 128)
        {
            return Err(AppError::Invalid("OCR normalization hint exceeds limit"));
        }
        if let Some(date) = &self.sampled_at {
            if date.len() > 64 {
                return Err(AppError::Invalid("Collection date exceeds limit"));
            }
            parse_time(date)?;
        }
        if let Some(metric) = &self.metric_id
            && !metric
                .strip_prefix("custom:")
                .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
            && !lab_metric_metadata::is_clinical_id(metric)
            && crate::laboratory::metric_definition(metric).is_none()
        {
            return Err(AppError::Invalid(
                "Unknown metric_id; leave unmapped instead",
            ));
        }
        if let Some(bounds) = self.source.bounding_box
            && (bounds
                .iter()
                .any(|coordinate| !coordinate.is_finite() || !(0.0..=1.0).contains(coordinate))
                || bounds[0] >= bounds[2]
                || bounds[1] >= bounds[3])
        {
            return Err(AppError::Invalid(
                "Bounding box must use normalized left, top, right, bottom",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportContext {
    pub report_title: Option<String>,
    pub source_name: Option<String>,
    pub fasting: Option<bool>,
    pub recent_exercise: Option<String>,
    pub illness: Option<String>,
    pub medications: Option<String>,
    pub notes: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ReportSummary {
    pub report_id: String,
    pub revision: i64,
    #[serde(skip_serializing)]
    pub context_json: String,
    pub original_name: String,
    pub page_count: i64,
    pub created_at: i64,
    pub collection_date: Option<String>,
}

#[derive(Serialize)]
pub struct Observation {
    pub observation_id: String,
    pub report_id: String,
    pub revision: i64,
    pub status: String,
    pub payload: ObservationPayload,
    pub interpretation: crate::laboratory::Interpretation,
    pub resolved_metric_id: Option<String>,
    pub display_name: String,
    pub lab_test_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lab_test_name: Option<&'static str>,
    pub browse_issues: Vec<BrowseIssue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reported_negative_bound: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric_suggestions: Option<Value>,
}

#[derive(Serialize)]
pub struct BrowseIssue {
    pub code: &'static str,
    pub scope: &'static str,
    pub message: String,
}

// A reported assay threshold is a review suggestion. The printed result remains intact.
fn negative_bound(payload: &ObservationPayload) -> Option<String> {
    let unit = payload.raw_unit.as_deref()?.trim();
    if unit.is_empty()
        || !matches!(
            payload.raw_result.trim().to_ascii_lowercase().as_str(),
            "negative" | "not detected"
        )
    {
        return None;
    }
    let bound = payload
        .notes
        .as_deref()?
        .lines()
        .find_map(|line| line.trim().strip_prefix("Reported negative threshold: <"))?;
    let (value, threshold_unit) = bound.trim().split_once(char::is_whitespace)?;
    let value = value.parse::<rust_decimal::Decimal>().ok()?;
    (value > rust_decimal::Decimal::ZERO
        && crate::laboratory::custom_unit(unit) == crate::laboratory::custom_unit(threshold_unit))
    .then(|| format!("<{value} {unit}"))
}

fn browse_issues(
    payload: &ObservationPayload,
    interpretation: &crate::laboratory::Interpretation,
    metric: Option<&str>,
    custom: Option<&CustomMetric>,
    candidates: &[crate::laboratory::MetricSuggestion<'_>],
) -> Vec<BrowseIssue> {
    let mut issues = Vec::new();
    if custom.is_none()
        && metric
            .and_then(crate::laboratory::metric_definition)
            .is_none()
    {
        let named = crate::laboratory::infer_metric(&payload.raw_name, None);
        let coded = crate::laboratory::split_test_code(&payload.raw_name)
            .1
            .and_then(crate::laboratory::report_suggestion_code);
        let candidate = candidates.iter().find(|candidate| {
            Some(candidate.metric_id) == payload.ocr_metric_candidate.as_deref()
                || Some(candidate.metric_id) == named
                || Some(candidate.metric_id) == coded
        });
        if crate::laboratory::test_code_conflicts(&payload.raw_name) {
            issues.push(BrowseIssue {
                code: "lab_test_code_conflict", scope: "browse",
                message: "The printed test name and test code do not identify the same metric. Review the original before mapping this result.".into(),
            });
        } else if let Some(candidate) = candidate.filter(|candidate| {
            !candidate.standard_unit.trim().is_empty()
                && candidate.unit_match == crate::laboratory::UnitMatch::Incompatible
        }) {
            let reported = payload.raw_unit.as_deref().unwrap_or("not specified");
            let standard = match candidate.standard_unit {
                "1" => "unitless (qualitative result)",
                unit => unit,
            };
            issues.push(BrowseIssue {
                code: "unsupported_unit",
                scope: "browse",
                message: format!(
                    "Unit mismatch · {}: report unit {reported}; database standard unit {standard}.",
                    candidate.name
                ),
            });
        } else if let Some(candidate) = candidate {
            issues.push(BrowseIssue {
                code: "metric_identity_uncertain",
                scope: "browse",
                message: format!(
                    "Suggested match: {}. Check the original report before mapping this result.",
                    candidate.name
                ),
            });
        } else {
            issues.push(BrowseIssue {
                code: "unmapped_metric",
                scope: "browse",
                message: "No existing Browse metric reliably matches this test. It appears separately for review. Check the original and select a metric if appropriate.".into(),
            });
        }
    } else if ["missing_unit", "unsupported_unit", "overflow"]
        .contains(&interpretation.result.status)
    {
        issues.push(BrowseIssue {
            code: interpretation.result.status,
            scope: "chart",
            message: format!(
                "Historical values only; cannot plot: {}",
                interpretation.result.reason
            ),
        });
    }
    if payload
        .raw_unit
        .as_deref()
        .is_some_and(|unit| !unit.trim().is_empty())
        && matches!(
            payload.raw_result.trim().to_ascii_lowercase().as_str(),
            "negative" | "not detected"
        )
    {
        issues.push(BrowseIssue {
            code: "negative_with_unit",
            scope: "review",
            message: negative_bound(payload).map_or_else(
                || "Verify the assay threshold in the original report before entering a numeric bound.".into(),
                |bound| format!("Reported negative threshold: {bound}. Verify against the original report."),
            ),
        });
    }
    match payload
        .sampled_at
        .as_deref()
        .filter(|date| !date.trim().is_empty())
    {
        None => issues.push(BrowseIssue {
            code: "missing_date",
            scope: "chart",
            message: "Historical values only; cannot plot without a collection date.".into(),
        }),
        Some(date) if parse_time(date).is_err() => issues.push(BrowseIssue {
            code: "invalid_date",
            scope: "chart",
            message: "Historical values only; the collection date is invalid.".into(),
        }),
        _ => {}
    }
    if payload.raw_result.trim().is_empty() {
        issues.push(BrowseIssue {
            code: "missing_result",
            scope: "chart",
            message: "Historical values only; no result value was provided.".into(),
        });
    }
    issues
}

#[derive(sqlx::FromRow)]
struct StoredObservation {
    observation_id: String,
    report_id: String,
    revision: i64,
    status: String,
    payload_json: String,
    context_json: String,
}
impl StoredObservation {
    fn decode(
        self,
        custom_metrics: &[CustomMetric],
        metadata: &Metadata,
        native_links: &std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
        include_suggestions: bool,
    ) -> Result<Observation, AppError> {
        let payload: ObservationPayload = serde_json::from_str(&self.payload_json)?;
        let context: Value = serde_json::from_str(&self.context_json)?;
        let code = test_code(&payload);
        let learned = code
            .and_then(|code| metadata.links.get(code))
            .map(String::as_str);
        let linked = code.and_then(crate::laboratory::loinc_metric).or(learned);
        let native = code.and_then(|code| native_links.get(code));
        let linked_candidate = linked.or_else(|| {
            native
                .filter(|ids| ids.len() == 1)
                .and_then(|ids| ids.first())
                .map(String::as_str)
        });
        let conflict = payload.metric_id.as_deref().is_some_and(|id| {
            linked.map_or_else(
                || native.is_some_and(|ids| ids.len() != 1 || !ids.contains(id)),
                |linked| linked != id,
            )
        });
        let resolved = if self.status == "pending"
            && learned.is_some()
            && payload.metric_id.is_none()
        {
            learned_for_pending(&payload, metadata)
        } else {
            crate::laboratory::resolved_report_metric(&payload, context["source_name"].as_str())
                .or_else(|| crate::laboratory::verified_custom_candidate(&payload, custom_metrics))
        };
        let resolved_metric_id = resolved
            .filter(|id| {
                !id.starts_with("custom:")
                    || custom_metrics.iter().any(|metric| metric.metric_id == *id)
            })
            .map(str::to_owned);
        let custom = custom_metrics
            .iter()
            .find(|metric| Some(metric.metric_id.as_str()) == resolved_metric_id.as_deref());
        let interpretation =
            crate::laboratory::interpret_metric(&payload, resolved_metric_id.as_deref(), custom);
        let candidates = if resolved_metric_id.is_none() || conflict {
            crate::laboratory::metric_suggestions(
                &payload.raw_name,
                payload.raw_unit.as_deref(),
                linked_candidate.or(payload.ocr_metric_candidate.as_deref()),
                custom_metrics,
            )
        } else {
            Vec::new()
        };
        let mut browse_issues = browse_issues(
            &payload,
            &interpretation,
            resolved_metric_id.as_deref(),
            custom,
            &candidates,
        );
        if conflict {
            let message = linked_candidate
                .map(|id| metadata.aliases.get(id).map(String::as_str)
                    .or_else(|| custom_metrics.iter().find(|metric| metric.metric_id == id).map(|metric| metric.name.as_str()))
                    .unwrap_or_else(|| crate::laboratory::display_name(id, Some(id))))
                .map_or_else(
                    || "This LOINC is used by multiple Apple Health metrics. Check the printed code and metric mapping.".into(),
                    |name| format!("This LOINC is already linked to {name}. Check the printed code and metric mapping."),
                );
            browse_issues.push(BrowseIssue {
                code: "loinc_metric_conflict",
                scope: "browse",
                message,
            });
        }
        let metric_suggestions =
            if include_suggestions && browse_issues.iter().any(|issue| issue.scope == "browse") {
                let mut suggestions = serde_json::to_value(candidates)?;
                if let Some(entries) = suggestions.as_array_mut() {
                    for entry in entries {
                        metadata.apply_name(entry);
                    }
                }
                Some(suggestions)
            } else {
                None
            };
        let (printed_name, lab_test_code) = crate::laboratory::split_test_code(&payload.raw_name);
        let saved_metric = resolved_metric_id
            .as_deref()
            .filter(|_| payload.metric_id.is_some() || self.status == "confirmed");
        let display_name = saved_metric.map_or(printed_name, |id| {
            metadata
                .aliases
                .get(id)
                .map(String::as_str)
                .or_else(|| custom.map(|metric| metric.name.as_str()))
                .unwrap_or_else(|| crate::laboratory::display_name(printed_name, Some(id)))
        });
        Ok(Observation {
            display_name: display_name.to_owned(),
            lab_test_code: lab_test_code.map(str::to_owned),
            lab_test_name: lab_test_code.and_then(crate::loinc::long_common_name),
            reported_negative_bound: negative_bound(&payload),
            browse_issues,
            metric_suggestions,
            observation_id: self.observation_id,
            report_id: self.report_id,
            revision: self.revision,
            status: self.status,
            payload,
            resolved_metric_id,
            interpretation,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewItem {
    pub observation_id: Option<String>,
    pub expected_revision: Option<i64>,
    pub status: String,
    pub payload: ObservationPayload,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewRequest {
    pub report_id: String,
    pub expected_revision: i64,
    pub context: Option<ReportContext>,
    pub observations: Vec<ReviewItem>,
    pub new_metric: Option<NewMetric>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NewMetric {
    pub name: String,
    pub standard_unit: String,
}

impl NewMetric {
    pub(crate) fn is_valid(&self) -> bool {
        let name = self.name.trim();
        !crate::laboratory_names::key(name).is_empty()
            && name.len() <= 160
            && self.standard_unit.len() <= 64
            && !name.chars().any(char::is_control)
            && !self.standard_unit.chars().any(char::is_control)
            && !crate::laboratory::is_administrative_name(name)
            && crate::laboratory::split_test_code(name).1.is_none()
    }
}

pub fn parse_time(input: &str) -> Result<i64, AppError> {
    if input.len() == 10 {
        return NaiveDate::parse_from_str(input, "%Y-%m-%d")
            .ok()
            .and_then(|date| date.and_hms_opt(0, 0, 0))
            .map(|date| date.and_utc().timestamp())
            .ok_or(AppError::Invalid("Date must be YYYY-MM-DD or RFC3339"));
    }
    DateTime::parse_from_rfc3339(input)
        .map(|date| date.timestamp())
        .map_err(|_| AppError::Invalid("Date must be YYYY-MM-DD or RFC3339"))
}

impl Database {
    pub async fn custom_metrics(&self, user_id: &str) -> Result<Vec<CustomMetric>, AppError> {
        Ok(sqlx::query_as(queries::CUSTOM_LAB_METRICS)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?)
    }

    pub async fn rescan_report(
        &self,
        user_id: &str,
        report_id: &str,
        revision: i64,
    ) -> Result<crate::models::Job, AppError> {
        self.report_summary(user_id, report_id).await?;
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let active: i64 = sqlx::query_scalar(queries::REPORT_ACTIVE_JOB)
            .bind(user_id)
            .bind(report_id)
            .fetch_one(&mut *transaction)
            .await?;
        if active != 0 {
            return Err(AppError::Conflict(
                "Report extraction is already in progress",
            ));
        }
        let updated = sqlx::query(queries::UPDATE_REPORT)
            .bind(Option::<String>::None)
            .bind(user_id)
            .bind(report_id)
            .bind(revision)
            .execute(&mut *transaction)
            .await?;
        if updated.rows_affected() != 1 {
            return Err(AppError::Conflict(
                "Report changed; reload before rescanning",
            ));
        }
        sqlx::query(queries::REPLACE_FINISHED_REPORT_JOB)
            .bind(user_id)
            .bind(report_id)
            .execute(&mut *transaction)
            .await?;
        let job_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(queries::INSERT_JOB)
            .bind(&job_id)
            .bind(user_id)
            .bind(report_id)
            .bind(now()?)
            .execute(&mut *transaction)
            .await?;
        sqlx::query(queries::MARK_RESCAN_JOB)
            .bind(&job_id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        self.job(user_id, &job_id).await
    }

    pub async fn report_summary(
        &self,
        user_id: &str,
        report_id: &str,
    ) -> Result<ReportSummary, AppError> {
        sqlx::query_as(queries::REPORT)
            .bind(user_id)
            .bind(report_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AppError::NotFound)
    }
    pub async fn reports(
        &self,
        user_id: &str,
        request: &ListRequest,
    ) -> Result<Vec<ReportSummary>, AppError> {
        request.validate()?;
        Ok(sqlx::query_as(queries::REPORTS)
            .bind(user_id)
            .bind(&request.after_id)
            .bind(&request.after_id)
            .bind(request.limit)
            .fetch_all(&self.pool)
            .await?)
    }
    pub async fn observations(
        &self,
        user_id: &str,
        report_id: &str,
    ) -> Result<Vec<Observation>, AppError> {
        self.report_summary(user_id, report_id).await?;
        let rows = sqlx::query_as::<_, StoredObservation>(queries::OBSERVATIONS)
            .bind(user_id)
            .bind(report_id)
            .fetch_all(&self.pool)
            .await?;
        self.decode_report_rows(
            user_id,
            rows,
            self.custom_metrics(user_id).await?,
            self.lab_metric_metadata(user_id).await?,
            false,
        )
        .await
    }
    pub async fn report(&self, user_id: &str, report_id: &str) -> Result<Value, AppError> {
        let mut transaction = self.pool.begin().await?;
        let report: ReportSummary = sqlx::query_as(queries::REPORT)
            .bind(user_id)
            .bind(report_id)
            .fetch_optional(&mut *transaction)
            .await?
            .ok_or(AppError::NotFound)?;
        let context: Value = serde_json::from_str(&report.context_json)?;
        let observations = sqlx::query_as::<_, StoredObservation>(queries::OBSERVATIONS)
            .bind(user_id)
            .bind(report_id)
            .fetch_all(&mut *transaction)
            .await?;
        let custom_metrics = sqlx::query_as(queries::CUSTOM_LAB_METRICS)
            .bind(user_id)
            .fetch_all(&mut *transaction)
            .await?;
        let metadata = Metadata::load(&mut transaction, user_id).await?;
        let relation=sqlx::query(queries::REPORT_RELATION).bind(user_id).bind(report_id).fetch_optional(&mut *transaction).await?
            .map(|row| json!({"preferred_report_id":row.get::<String,_>("preferred_report_id"),"kind":row.get::<String,_>("kind")}));
        let pages=sqlx::query(queries::EXTRACTION_PAGES).bind(user_id).bind(report_id).fetch_all(&mut *transaction).await?.into_iter()
            .map(|row|json!({"run_id":row.get::<String,_>("run_id"),"page":row.get::<i64,_>("page_number"),"status":row.get::<String,_>("status"),
                "content":row.get::<String,_>("content"),"model":row.get::<String,_>("model"),"created_at":row.get::<i64,_>("created_at")})).collect::<Vec<_>>();
        let outputs = sqlx::query(queries::EXTRACTION_OUTPUTS).bind(user_id).bind(report_id)
            .fetch_all(&mut *transaction).await?.into_iter().map(|row| json!({
                "run_id":row.get::<String,_>("run_id"), "page":row.get::<i64,_>("page_number"),
                "stage":row.get::<String,_>("stage"), "model":row.get::<String,_>("model"),
                "adapter":row.get::<String,_>("adapter"), "created_at":row.get::<i64,_>("created_at")
            })).collect::<Vec<_>>();
        let inputs = sqlx::query(queries::EXTRACTION_INPUTS).bind(user_id).bind(report_id).fetch_all(&mut *transaction).await?
            .into_iter().map(|row| json!({"run_id":row.get::<String,_>("run_id"),"page":row.get::<i64,_>("page_number"),"created_at":row.get::<i64,_>("created_at"),"text_status":row.get::<Option<String>,_>("text_status"),"error_code":row.get::<Option<String>,_>("error_code")})).collect::<Vec<_>>();
        let duplicates = sqlx::query(queries::DUPLICATE_FILES).bind(user_id).bind(user_id).bind(report_id).bind(report_id).fetch_all(&mut *transaction).await?.into_iter().map(|row|json!({"report_id":row.get::<String,_>("file_id"),"original_name":row.get::<String,_>("original_name"),"reason":"identical_file_bytes"})).collect::<Vec<_>>();
        transaction.commit().await?;
        let observations = self
            .decode_report_rows(user_id, observations, custom_metrics, metadata, true)
            .await?;
        let evidence = sqlx::query(queries::EXTRACTION_VALIDATION_CONTENT)
            .bind(user_id)
            .bind(report_id)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|row| {
                (
                    row.get::<i64, _>("page_number"),
                    row.get::<Option<String>, _>("content"),
                    row.get::<bool, _>("has_report_collection_date"),
                )
            })
            .collect::<Vec<_>>();
        let validation_issues = self
            .cpu
            .run(move || {
                Ok(evidence
                    .into_iter()
                    .flat_map(|(page, content, has_report_collection_date)| {
                        crate::ocr::validation_issues(
                            content.as_deref().unwrap_or(""),
                            page,
                            has_report_collection_date,
                        )
                    })
                    .collect::<Vec<_>>())
            })
            .await?;
        Ok(
            json!({"validation_issues":validation_issues,"duplicate_candidates":duplicates,"report":report,"context":context,"observations":observations,"relation":relation,"pages":pages,"extraction_outputs":outputs,"extraction_inputs":inputs}),
        )
    }
    pub async fn extraction_input(
        &self,
        user_id: &str,
        request: ExtractionInputRequest,
    ) -> Result<Value, AppError> {
        let body: String = sqlx::query_scalar(queries::EXTRACTION_INPUT)
            .bind(user_id)
            .bind(request.report_id)
            .bind(request.run_id)
            .bind(request.page)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AppError::NotFound)?;
        self.cpu.run(move || Ok(serde_json::from_str(&body)?)).await
    }
    pub async fn extraction_output(
        &self,
        user_id: &str,
        request: ExtractionOutputRequest,
    ) -> Result<Value, AppError> {
        let row = sqlx::query(queries::EXTRACTION_OUTPUT)
            .bind(user_id)
            .bind(request.report_id)
            .bind(request.run_id)
            .bind(request.page)
            .bind(request.stage)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AppError::NotFound)?;
        Ok(
            json!({"response_body":row.get::<String,_>("response_body"), "status_code":row.get::<i64,_>("status_code"), "content":row.get::<Option<String>,_>("content")}),
        )
    }
    /// Fill only absent metadata. Review uses revision checks and relocates the original atomically.
    pub async fn autofill_report_identity(
        &self,
        user_id: &str,
        report_id: &str,
        title: Option<String>,
        source: Option<String>,
    ) -> Result<(), AppError> {
        if title.is_none() && source.is_none() {
            return Ok(());
        }
        for attempt in 0..3 {
            let report = self.report_summary(user_id, report_id).await?;
            let mut context: ReportContext = serde_json::from_str(&report.context_json)?;
            let mut changed = false;
            if context.report_title.is_none() && title.is_some() {
                context.report_title = title.clone();
                changed = true;
            }
            if context.source_name.is_none() && source.is_some() {
                context.source_name = source.clone();
                changed = true;
            }
            if !changed {
                return Ok(());
            }
            match self
                .review(
                    user_id,
                    ReviewRequest {
                        new_metric: None,
                        report_id: report_id.into(),
                        expected_revision: report.revision,
                        context: Some(context),
                        observations: vec![],
                    },
                )
                .await
            {
                Ok(_) => return Ok(()),
                Err(AppError::Conflict(_)) if attempt < 2 => continue,
                Err(error) => return Err(error),
            }
        }
        Err(AppError::Conflict("Report changed during automatic naming"))
    }
    pub async fn review(&self, user_id: &str, request: ReviewRequest) -> Result<Value, AppError> {
        self.review_with_analysis(user_id, request, false).await
    }

    pub(crate) async fn review_with_analysis(
        &self,
        user_id: &str,
        request: ReviewRequest,
        analysis_enabled: bool,
    ) -> Result<Value, AppError> {
        let queue_analysis = analysis_enabled
            && request
                .observations
                .iter()
                .any(|item| item.status == "confirmed");
        let report = self.report_summary(user_id, &request.report_id).await?;
        if request.new_metric.is_some()
            && (request.observations.len() != 1
                || request.observations[0].observation_id.is_none()
                || request.observations[0].status == "rejected"
                || request.observations[0].payload.metric_id.is_some())
        {
            return Err(AppError::Invalid(
                "Create a metric while reviewing one existing result",
            ));
        }
        if request.observations.len() > 2000 {
            return Err(AppError::TooLarge);
        }
        let mut seen = HashSet::new();
        for observation in &request.observations {
            observation.payload.validate(report.page_count)?;
            if !["pending", "confirmed", "rejected"].contains(&observation.status.as_str()) {
                return Err(AppError::Invalid("Invalid review status"));
            }
            if let Some(id) = &observation.observation_id {
                if !seen.insert(id) {
                    return Err(AppError::Invalid("Duplicate observation in review"));
                }
                if observation.expected_revision.is_none() {
                    return Err(AppError::Invalid("Observation revision required"));
                }
            } else if observation.expected_revision.is_some() {
                return Err(AppError::Invalid("New observation cannot have a revision"));
            }
        }
        if let Some(name) = request
            .context
            .as_ref()
            .and_then(|context| context.source_name.as_ref())
            && (name.trim().is_empty()
                || name.len() > 160
                || name.chars().any(char::is_control)
                || name.trim().eq_ignore_ascii_case("Apple Health"))
        {
            return Err(AppError::Invalid(
                "Use a clinic or laboratory name of 1-160 bytes; Apple Health is reserved",
            ));
        }
        if request
            .context
            .as_ref()
            .and_then(|c| c.report_title.as_ref())
            .is_some_and(|title| {
                title.trim().is_empty() || title.len() > 256 || title.chars().any(char::is_control)
            })
        {
            return Err(AppError::Invalid(
                "Report title must contain 1-256 bytes without control characters",
            ));
        }
        let context = request
            .context
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        if context.as_ref().is_some_and(|text| text.len() > 16384) {
            return Err(AppError::TooLarge);
        }
        let needs_clinical_catalog = request.observations.iter().any(|item| {
            item.payload
                .metric_id
                .as_deref()
                .is_some_and(lab_metric_metadata::is_clinical_id)
                || (item.status == "confirmed"
                    && item.payload.metric_id.is_some()
                    && test_code(&item.payload)
                        .is_some_and(|code| crate::laboratory::loinc_metric(code).is_none()))
        });
        let clinical = if needs_clinical_catalog {
            lab_metric_metadata::clinical_groups(self, user_id).await?
        } else {
            Default::default()
        };
        let native_links = lab_metric_metadata::clinical_links(&clinical);
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let updated = sqlx::query(queries::UPDATE_REPORT)
            .bind(context)
            .bind(user_id)
            .bind(&request.report_id)
            .bind(request.expected_revision)
            .execute(&mut *transaction)
            .await?;
        if updated.rows_affected() != 1 {
            return Err(AppError::Conflict(
                "Report changed; reload before reviewing",
            ));
        }
        if let Some(context) = request.context.as_ref() {
            let original = sqlx::query(queries::REPORT_ORIGINAL)
                .bind(user_id)
                .bind(user_id)
                .bind(&request.report_id)
                .fetch_one(&mut *transaction)
                .await?;
            let file_id: String = original.get("file_id");
            let old: String = original.get("relative_path");
            let new = crate::raw::document_path(
                user_id,
                &file_id,
                &original.get::<String, _>("content_type"),
                context.source_name.as_deref(),
            );
            if new != old {
                let target = self.data_dir.join(&new);
                tokio::fs::create_dir_all(target.parent().ok_or(AppError::Internal)?).await?;
                // Link before commit; keep the old original until its cleanup task runs.
                // A failed transaction leaves only an unreferenced link for orphan recovery.
                match tokio::fs::hard_link(self.data_dir.join(&old), &target).await {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                        let old_bytes = tokio::fs::read(self.data_dir.join(&old)).await?;
                        if tokio::fs::read(&target).await? != old_bytes {
                            return Err(AppError::Conflict("Original destination differs"));
                        }
                    }
                    Err(error) => return Err(error.into()),
                }
                crate::raw::sync_archive_directories(&self.data_dir, &[&new]).await?;
                sqlx::query(queries::RELOCATE_ORIGINAL)
                    .bind(&new)
                    .bind(user_id)
                    .bind(&file_id)
                    .execute(&mut *transaction)
                    .await?;
                // Cancel an older cleanup when a source is renamed back. The API
                // serializes relocation with cleanup through the file mutation lock.
                sqlx::query(queries::CANCEL_PATH_CLEANUP)
                    .bind(&new)
                    .execute(&mut *transaction)
                    .await?;
                sqlx::query(queries::INSERT_CLEANUP)
                    .bind(uuid::Uuid::new_v4().to_string())
                    .bind(old)
                    .bind(now()?)
                    .execute(&mut *transaction)
                    .await?;
            }
        }
        let created_metric = if let Some(metric) = request.new_metric {
            let name = metric.name.trim();
            let name_key = crate::laboratory_names::key(name);
            let standard_unit = crate::laboratory::custom_unit(&metric.standard_unit);
            if !metric.is_valid() {
                return Err(AppError::Invalid(
                    "Use a metric name without the printed test code and a unit of up to 64 bytes",
                ));
            }
            if crate::laboratory::infer_metric(
                name,
                (!standard_unit.is_empty()).then_some(standard_unit),
            )
            .is_some()
            {
                return Err(AppError::Conflict(
                    "A matching Browse metric already exists. Select it instead.",
                ));
            }
            let custom = CustomMetric {
                metric_id: format!("custom:{}", uuid::Uuid::new_v4()),
                name: name.to_owned(),
                standard_unit: standard_unit.to_owned(),
            };
            let inserted = sqlx::query(queries::INSERT_CUSTOM_LAB_METRIC)
                .bind(&custom.metric_id)
                .bind(user_id)
                .bind(&custom.name)
                .bind(name_key)
                .bind(&custom.standard_unit)
                .execute(&mut *transaction)
                .await?;
            if inserted.rows_affected() != 1 {
                return Err(AppError::Conflict(
                    "You already have a metric with this name and unit",
                ));
            }
            Some(custom)
        } else {
            None
        };
        let custom_metrics: Vec<CustomMetric> = sqlx::query_as(queries::CUSTOM_LAB_METRICS)
            .bind(user_id)
            .fetch_all(&mut *transaction)
            .await?;
        let mut metadata = Metadata::load(&mut transaction, user_id).await?;
        for mut observation in request.observations {
            if let Some(metric) = &created_metric {
                observation.payload.metric_id = Some(metric.metric_id.clone());
            }
            if let Some(id) = observation
                .payload
                .metric_id
                .as_deref()
                .filter(|id| id.starts_with("custom:"))
                && !custom_metrics.iter().any(|metric| metric.metric_id == id)
            {
                return Err(AppError::Invalid(
                    "Custom metric is not available for this account",
                ));
            }
            if let Some(id) = observation.payload.metric_id.as_deref()
                && lab_metric_metadata::is_clinical_id(id)
                && !clinical.contains_key(id)
            {
                return Err(AppError::Invalid(
                    "Apple Health metric is not available for this account",
                ));
            }
            let (id, revision, previous) = if let Some(id) = observation.observation_id {
                let current: Option<(i64, String, String)> = sqlx::query_as(queries::OBSERVATION)
                    .bind(user_id)
                    .bind(&request.report_id)
                    .bind(&id)
                    .fetch_optional(&mut *transaction)
                    .await?;
                let (revision, status, payload) = current.ok_or(AppError::NotFound)?;
                if Some(revision) != observation.expected_revision {
                    return Err(AppError::Conflict("Observation changed"));
                }
                let payload: ObservationPayload = serde_json::from_str(&payload)?;
                (id, revision + 1, Some((status, payload)))
            } else {
                let id = uuid::Uuid::new_v4().to_string();
                sqlx::query(queries::INSERT_OBSERVATION)
                    .bind(&id)
                    .bind(user_id)
                    .bind(&request.report_id)
                    .bind(Option::<String>::None)
                    .execute(&mut *transaction)
                    .await?;
                (id, 1, None)
            };
            if observation.status == "confirmed"
                && previous.as_ref().is_none_or(|(status, payload)| {
                    status != "confirmed"
                        || payload.metric_id != observation.payload.metric_id
                        || test_code(payload) != test_code(&observation.payload)
                })
            {
                if observation.payload.metric_id.is_some() {
                    // Explicit mappings come from the reviewer. OCR cannot supply this field.
                    lab_metric_metadata::link_confirmed(
                        &mut transaction,
                        user_id,
                        &mut metadata,
                        &native_links,
                        &observation.payload,
                    )
                    .await?;
                } else if let Some(id) = learned_for_pending(&observation.payload, &metadata) {
                    // Freeze the association when the result becomes part of the history.
                    observation.payload.metric_id = Some(id.to_owned());
                } else if test_code(&observation.payload)
                    .is_some_and(|code| metadata.links.contains_key(code))
                {
                    return Err(AppError::Invalid(
                        "This result needs review before confirming. Check the source and select the linked Browse metric.",
                    ));
                }
            }
            let statement = if observation.status == "confirmed" {
                queries::CONFIRM_OBSERVATION
            } else {
                queries::SAVE_OBSERVATION
            };
            sqlx::query(statement)
                .bind(user_id)
                .bind(id)
                .bind(revision)
                .bind(observation.status)
                .bind(serde_json::to_string(&observation.payload)?)
                .bind(now()?)
                .execute(&mut *transaction)
                .await?;
        }
        sqlx::query(queries::TOUCH_USER_DATA)
            .bind(user_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query(queries::INVALIDATE_ANALYSES)
            .bind(user_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query(queries::INVALIDATE_EXPORTS)
            .bind(user_id)
            .execute(&mut *transaction)
            .await?;
        if queue_analysis {
            sqlx::query(queries::QUEUE_REPORT_ANALYSIS)
                .bind(user_id)
                .bind(uuid::Uuid::new_v4().to_string())
                .bind(now()?)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        self.reconcile_report_drafts().await?;
        let mut document = self.report(user_id, &request.report_id).await?;
        if let Some(metric) = created_metric {
            document["created_metric"] = serde_json::to_value(metric)?;
        }
        if queue_analysis {
            document["analysis_trigger"] = json!({"status":"queued"});
        }
        Ok(document)
    }
    pub async fn observation_history(
        &self,
        user_id: &str,
        observation_id: &str,
    ) -> Result<Value, AppError> {
        let rows = sqlx::query(queries::OBSERVATION_HISTORY)
            .bind(user_id)
            .bind(observation_id)
            .fetch_all(&self.pool)
            .await?;
        if rows.is_empty() {
            return Err(AppError::NotFound);
        }
        let history=rows.into_iter().map(|row| Ok(json!({"revision":row.get::<i64,_>("revision"),"status":row.get::<String,_>("status"),
            "payload":serde_json::from_str::<Value>(&row.get::<String,_>("payload_json"))?,"created_at":row.get::<i64,_>("created_at")})))
            .collect::<Result<Vec<_>,AppError>>()?;
        Ok(json!({"history":history}))
    }
    pub async fn confirmed(&self, user_id: &str) -> Result<Vec<Observation>, AppError> {
        let rows = sqlx::query_as::<_, StoredObservation>(queries::CONFIRMED_OBSERVATIONS)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        if rows.len() > 10000 {
            return Err(AppError::TooLarge);
        }
        let mut sources = std::collections::HashMap::new();
        for row in &rows {
            if !sources.contains_key(&row.report_id) {
                let context: Value = serde_json::from_str(&row.context_json)?;
                let source =
                    crate::sources::report_key(context["source_name"].as_str(), &row.report_id);
                sources.insert(row.report_id.clone(), source);
            }
        }
        let mut observations = self
            .decode_report_rows(
                user_id,
                rows,
                self.custom_metrics(user_id).await?,
                self.lab_metric_metadata(user_id).await?,
                false,
            )
            .await?;
        let mut first_report = std::collections::HashMap::new();
        observations.retain(|row| {
            crate::report_dedup::result_key(&row.payload, row.resolved_metric_id.as_deref(), None)
                .is_none_or(|key| {
                    first_report
                        .entry((&sources[&row.report_id], key))
                        .or_insert_with(|| row.report_id.clone())
                        == &row.report_id
                })
        });
        Ok(observations)
    }
    pub async fn relate_reports(
        &self,
        user_id: &str,
        report_id: &str,
        preferred: Option<&str>,
        kind: &str,
        revision: i64,
    ) -> Result<(), AppError> {
        self.report_summary(user_id, report_id).await?;
        if let Some(preferred) = preferred {
            if preferred == report_id || !["duplicate", "superseded"].contains(&kind) {
                return Err(AppError::Invalid("Invalid report relationship"));
            }
            self.report_summary(user_id, preferred).await?;
        }
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let updated = sqlx::query(queries::UPDATE_REPORT)
            .bind(Option::<String>::None)
            .bind(user_id)
            .bind(report_id)
            .bind(revision)
            .execute(&mut *transaction)
            .await?;
        if updated.rows_affected() != 1 {
            return Err(AppError::Conflict("Report changed"));
        }
        if let Some(preferred) = preferred {
            let depends: i64 = sqlx::query_scalar(queries::RELATION_DEPENDENTS)
                .bind(user_id)
                .bind(report_id)
                .fetch_one(&mut *transaction)
                .await?;
            let excluded = sqlx::query(queries::REPORT_RELATION)
                .bind(user_id)
                .bind(preferred)
                .fetch_optional(&mut *transaction)
                .await?;
            if depends > 0 || excluded.is_some() {
                return Err(AppError::Conflict(
                    "Choose a primary report; chained relations are not allowed",
                ));
            }
            sqlx::query(queries::SET_RELATION)
                .bind(user_id)
                .bind(report_id)
                .bind(preferred)
                .bind(kind)
                .execute(&mut *transaction)
                .await?;
        } else {
            sqlx::query(queries::CLEAR_RELATION)
                .bind(user_id)
                .bind(report_id)
                .execute(&mut *transaction)
                .await?;
        }
        sqlx::query(queries::TOUCH_USER_DATA)
            .bind(user_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query(queries::INVALIDATE_ANALYSES)
            .bind(user_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query(queries::INVALIDATE_EXPORTS)
            .bind(user_id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }
}

pub(crate) fn history_record(observation: &Observation) -> Value {
    json!({"point_id":format!("report:{}", observation.observation_id),
            "observation_id":observation.observation_id,"report_id":observation.report_id,
            "sampled_at":observation.payload.sampled_at,"display_value":observation.payload.raw_result,
            "unit":observation.payload.raw_unit,"status":observation.status,"original":observation.payload})
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrendRequest {
    pub metric_ids: Vec<String>,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
}
pub async fn trends(
    database: &Database,
    user_id: &str,
    request: TrendRequest,
) -> Result<Value, AppError> {
    if request.metric_ids.is_empty() || request.metric_ids.len() > 20 {
        return Err(AppError::Invalid("Select 1-20 metrics"));
    }
    let start = request
        .start_at
        .as_deref()
        .map(parse_time)
        .transpose()?
        .unwrap_or(i64::MIN);
    let end = request
        .end_at
        .as_deref()
        .map(parse_time)
        .transpose()?
        .unwrap_or(i64::MAX);
    if start > end {
        return Err(AppError::Invalid("Invalid time range"));
    }
    let mut points = Vec::new();
    let mut records = Vec::new();
    let mut incomparable = 0;
    let mut excluded = Vec::new();
    for observation in database.confirmed(user_id).await? {
        if !observation
            .resolved_metric_id
            .as_ref()
            .is_some_and(|id| request.metric_ids.contains(id))
        {
            continue;
        }
        records.push(history_record(&observation));
        let Some(date) = observation.payload.sampled_at.as_deref() else {
            excluded.push(json!({"observation_id":observation.observation_id,"reason":"missing_collection_date"}));
            incomparable += 1;
            continue;
        };
        let timestamp = parse_time(date)?;
        if timestamp < start || timestamp > end {
            continue;
        }
        let quantity = &observation.interpretation.result;
        let Some((number, unit)) = (quantity.status == "exact")
            .then(|| quantity.value.as_ref().zip(quantity.unit.as_ref()))
            .flatten()
        else {
            excluded.push(json!({"observation_id":observation.observation_id,"reason":observation.interpretation.result.status}));
            incomparable += 1;
            continue;
        };
        points.push(json!({"observation_id":observation.observation_id,"report_id":observation.report_id,"revision":observation.revision,
            "metric_id":observation.resolved_metric_id,"sampled_at":date,"timestamp":timestamp,"date_precision":if date.len()==10{"day"}else{"second"},
            "value":number,"unit":unit,"reference":observation.interpretation.reference,"original":observation.payload}));
    }
    points.sort_by_key(|point| point["timestamp"].as_i64());
    Ok(
        json!({"records":records,"points":points,"incomparable_count":incomparable,"excluded":excluded,"algorithm_version":"lab-trend-v1","conversion_version":crate::laboratory::CONVERSION_VERSION}),
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractionOutputRequest {
    pub report_id: String,
    pub run_id: String,
    pub page: i64,
    pub stage: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractionInputRequest {
    pub report_id: String,
    pub run_id: String,
    pub page: i64,
}

pub(crate) fn learned_for_pending<'a>(
    payload: &ObservationPayload,
    metadata: &'a Metadata,
) -> Option<&'a str> {
    if payload.metric_id.is_some()
        || payload
            .ocr_mapping_note
            .as_ref()
            .is_some_and(|note| !note.trim().is_empty())
    {
        return None;
    }
    let code = test_code(payload)?;
    let id = metadata.links.get(code)?;
    if crate::laboratory::loinc_metric(code).is_some_and(|builtin| builtin != id)
        || payload
            .ocr_metric_candidate
            .as_ref()
            .is_some_and(|candidate| candidate != id)
    {
        return None;
    }
    Some(id)
}

impl Database {
    async fn decode_report_rows(
        &self,
        user: &str,
        rows: Vec<StoredObservation>,
        mut custom_metrics: Vec<CustomMetric>,
        metadata: Metadata,
        include_suggestions: bool,
    ) -> Result<Vec<Observation>, AppError> {
        let needs_review_catalog = |payload: &ObservationPayload| {
            payload
                .metric_id
                .as_deref()
                .is_some_and(lab_metric_metadata::is_clinical_id)
                || (include_suggestions
                    && (payload
                        .ocr_metric_candidate
                        .as_deref()
                        .is_some_and(lab_metric_metadata::is_clinical_id)
                        || (payload.metric_id.is_some()
                            && test_code(payload).is_some_and(|code| {
                                crate::laboratory::loinc_metric(code).is_none()
                            }))))
        };
        let clinical = if rows.iter().any(|row| {
            serde_json::from_str(&row.payload_json).is_ok_and(|payload| {
                needs_review_catalog(&payload)
                    || (row.status == "pending"
                        && test_code(&payload)
                            .and_then(|code| metadata.links.get(code))
                            .is_some_and(|id| lab_metric_metadata::is_clinical_id(id)))
            })
        }) {
            lab_metric_metadata::clinical_groups(self, user).await?
        } else {
            Default::default()
        };
        let native_links = lab_metric_metadata::clinical_links(&clinical);
        for (id, group) in clinical {
            if lab_metric_metadata::is_clinical_id(&id) {
                custom_metrics.push(lab_metric_metadata::clinical_metric(&group)?);
            }
        }
        self.cpu
            .run(move || {
                rows.into_iter()
                    .map(|row| {
                        row.decode(
                            &custom_metrics,
                            &metadata,
                            &native_links,
                            include_suggestions,
                        )
                    })
                    .filter(|row| {
                        !row.as_ref().is_ok_and(|o| {
                            crate::laboratory::is_administrative_name(&o.payload.raw_name)
                        })
                    })
                    .collect()
            })
            .await
    }
}
