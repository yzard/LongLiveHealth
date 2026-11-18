use crate::{
    error::AppError, laboratory::CustomMetric, provider::Completion, reports::ObservationPayload,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExtractedPage {
    pub(crate) report_title: Option<String>,
    pub(crate) source_name: Option<String>,
    pub(crate) observations: Vec<ObservationPayload>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ModelPage {
    Report(ExtractedPage),
    Observations(Vec<ObservationPayload>),
}

impl ModelPage {
    fn parse(text: &str) -> Option<ExtractedPage> {
        let mut page = crate::provider::parse_json::<Value>(text).ok()?;
        let rows = match &mut page {
            Value::Array(rows) => Some(rows),
            Value::Object(object) => object.get_mut("observations")?.as_array_mut(),
            _ => None,
        }?;
        let mut invalid_hints = 0;
        for row in rows.iter_mut().filter_map(Value::as_object_mut) {
            let new_metric =
                discard_invalid_hint::<crate::reports::NewMetric>(row, "ocr_new_metric");
            let normalization =
                discard_invalid_hint::<crate::reports::OcrNormalization>(row, "ocr_normalization");
            invalid_hints += usize::from(new_metric || normalization);
        }
        let mut parsed = serde_json::from_value::<Self>(page)
            .ok()?
            .into_extracted_page();
        if invalid_hints > 0 {
            parsed.warnings.push(format!(
                "Ignored optional OCR suggestions with an invalid format on {invalid_hints} result rows."
            ));
        }
        Some(parsed)
    }

    fn into_extracted_page(self) -> ExtractedPage {
        match self {
            Self::Report(report) => report,
            Self::Observations(observations) => ExtractedPage {
                report_title: None,
                source_name: None,
                observations,
                warnings: vec!["The model returned result rows without page metadata.".into()],
            },
        }
    }
}

fn discard_invalid_hint<T: DeserializeOwned>(
    row: &mut serde_json::Map<String, Value>,
    key: &str,
) -> bool {
    if row
        .get(key)
        .is_some_and(|hint| !hint.is_null() && T::deserialize(hint).is_err())
    {
        row.remove(key);
        return true;
    }
    false
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractionResponse {
    pub report_title: Option<String>,
    pub source_name: Option<String>,
    pub model: String,
    pub engine: String,
    pub prompt_version: String,
    pub content: String,
    pub raw_response_body: String,
    pub observations: Vec<ObservationPayload>,
    pub warnings: Vec<String>,
}

#[derive(Default)]
pub struct MetricCatalog {
    pub metrics: Vec<CustomMetric>,
    pub loinc_codes: BTreeMap<String, BTreeSet<String>>,
    pub browse_metric_index: BTreeMap<String, BTreeSet<String>>,
    pub printed_aliases: BTreeMap<String, String>,
}

pub(crate) async fn database_metrics(
    database: &crate::database::Database,
    user_id: &str,
) -> Result<Arc<MetricCatalog>, AppError> {
    // Submitting a result writes the definition and its LOINC link together.
    let mut snapshot = database.pool.begin().await?;
    let metadata = crate::lab_metric_metadata::Metadata::load(&mut snapshot, user_id).await?;
    let stored: Vec<CustomMetric> = sqlx::query_as(crate::database::queries::CUSTOM_LAB_METRICS)
        .bind(user_id)
        .fetch_all(&mut *snapshot)
        .await?;
    snapshot.commit().await?;
    let mut loinc_codes = metadata.linked_codes_by_metric();
    let mut metrics: BTreeMap<_, _> = stored
        .into_iter()
        .map(|metric| (metric.metric_id.clone(), metric))
        .collect();
    for (id, group) in crate::lab_metric_metadata::clinical_groups(database, user_id).await? {
        metrics.insert(
            id.clone(),
            crate::lab_metric_metadata::clinical_metric(&group)?,
        );
        loinc_codes.entry(id).or_default().extend(group.loinc_codes);
    }
    for metric in metadata.named_report_metrics() {
        metrics.insert(metric.metric_id.clone(), metric);
    }
    let mut printed_aliases: BTreeMap<_, _> = metadata
        .named_reports
        .iter()
        .map(|(id, metric)| (id.clone(), metric.printed_name.clone()))
        .collect();
    for (id, metric) in &metadata.named_reports {
        loinc_codes
            .entry(id.clone())
            .or_default()
            .extend(metric.loinc_code.iter().cloned());
    }
    for (id, name) in metadata.aliases {
        if let Some(entry) = metrics.get_mut(&id) {
            printed_aliases
                .entry(id)
                .or_insert_with(|| entry.name.clone());
            entry.name = name;
        } else if let Some(metric) = crate::laboratory::metrics()
            .into_iter()
            .find(|metric| metric.metric_id == id)
        {
            printed_aliases.insert(id.clone(), metric.name.into());
            metrics.insert(
                id.clone(),
                CustomMetric {
                    metric_id: id,
                    name,
                    standard_unit: metric.standard_unit.into(),
                },
            );
        }
    }
    let mut browse_metric_index = BTreeMap::<String, BTreeSet<String>>::new();
    let catalog = crate::browse_catalog::get(database, user_id).await?;
    for entry in catalog["entries"].as_array().ok_or(AppError::Internal)? {
        let name = entry["title"].as_str().ok_or(AppError::Internal)?;
        let codes = browse_metric_index.entry(name.to_owned()).or_default();
        let definition = &entry["definition"];
        codes.extend(
            definition["loinc_codes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned),
        );
        codes.extend(
            definition["loinc"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|loinc| loinc["code"].as_str())
                .map(str::to_owned),
        );
        if let Some(id) = definition["metric_id"].as_str() {
            codes.extend(loinc_codes.get(id).into_iter().flatten().cloned());
            codes.extend(
                crate::laboratory::LOINC_METRICS
                    .iter()
                    .filter(|&&(_, metric)| metric == id)
                    .map(|&(code, _)| code.to_owned()),
            );
        }
    }
    Ok(Arc::new(MetricCatalog {
        metrics: metrics.into_values().collect(),
        loinc_codes,
        browse_metric_index,
        printed_aliases,
    }))
}

pub(crate) async fn extract(
    cpu: &crate::execution::CpuExecutor,
    config: &crate::config::Config,
    page: i64,
    image_url: String,
    evidence: &crate::documents::PageEvidence,
    catalog_metrics: Arc<MetricCatalog>,
) -> Result<Completion, AppError> {
    let budget = config.extraction.image_pixel_budget;
    let maximum_image_pixels = config.storage.maximum_image_pixels;
    let prompt_metrics = catalog_metrics.clone();
    let text_layer = evidence.text_layer.clone();
    let report_collection_date = evidence.report_collection_date.clone();
    let messages = cpu
        .run_background(move || {
            prepare_messages(
                page,
                &image_url,
                text_layer,
                report_collection_date,
                budget,
                maximum_image_pixels,
                &prompt_metrics,
            )
        })
        .await?;
    let mut provider = config.llm.clone();
    provider
        .extra_body
        .get_or_insert_with(Default::default)
        .insert("max_tokens".into(), json!(config.extraction.output_tokens));
    let reply = crate::provider::complete_once(&provider, messages).await?;
    let model = provider.model.clone();
    cpu.run_background(move || wrap_completion(reply, &model, page, &catalog_metrics.metrics))
        .await
}

pub fn prepare_messages(
    page: i64,
    image_url: &str,
    text_layer: Option<crate::documents::TextLayer>,
    report_collection_date: Option<crate::documents::ReportCollectionDate>,
    pixel_budget: u64,
    maximum_pixels: u64,
    catalog: &MetricCatalog,
) -> Result<serde_json::Value, AppError> {
    use base64::Engine;
    use image::ImageDecoder;
    use std::io::Cursor;
    let (prefix, encoded) = image_url
        .split_once(',')
        .ok_or(AppError::Invalid("Invalid inline page image"))?;
    if !["data:image/png;base64", "data:image/jpeg;base64"].contains(&prefix) {
        return Err(AppError::Invalid("Inline PNG or JPEG required"));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| AppError::Invalid("Invalid page encoding"))?;
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| AppError::Invalid("Invalid page image"))?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(maximum_pixels.saturating_mul(8));
    reader.limits(limits);
    let mut decoder = reader
        .into_decoder()
        .map_err(|_| AppError::Invalid("Cannot decode page image"))?;
    let (width, height) = decoder.dimensions();
    if u64::from(width) * u64::from(height) > maximum_pixels {
        return Err(AppError::TooLarge);
    }
    let orientation = decoder
        .orientation()
        .map_err(|_| AppError::Invalid("Cannot read page orientation"))?;
    let mut image = image::DynamicImage::from_decoder(decoder)
        .map_err(|_| AppError::Invalid("Cannot decode page image"))?;
    image.apply_orientation(orientation);
    let scale = (pixel_budget as f64 / (f64::from(image.width()) * f64::from(image.height())))
        .sqrt()
        .min(1.);
    if scale < 1. {
        image = image.resize_exact(
            (f64::from(image.width()) * scale).floor().max(1.) as u32,
            (f64::from(image.height()) * scale).floor().max(1.) as u32,
            image::imageops::FilterType::Lanczos3,
        );
    }
    let mut png = Cursor::new(Vec::new());
    image
        .write_to(&mut png, image::ImageFormat::Png)
        .map_err(|_| AppError::Invalid("Cannot encode page image"))?;
    let printed_loinc_names: BTreeMap<_, _> = text_layer
        .iter()
        .flat_map(|layer| {
            layer
                .text
                .split(|character: char| !character.is_ascii_digit() && character != '-')
        })
        .filter_map(|code| crate::loinc::long_common_name(code).map(|name| (code, name)))
        .take(256)
        .collect();
    let laboratory_metric_catalog = crate::laboratory::ocr_metric_catalog(
        &catalog.metrics,
        &catalog.loinc_codes,
        &catalog.printed_aliases,
    );
    let catalog_loinc_names: BTreeMap<_, _> = laboratory_metric_catalog
        .iter()
        .flat_map(|entry| entry.4.iter().copied())
        .filter_map(|code| crate::loinc::long_common_name(code).map(|name| (code, name)))
        .collect();
    let evidence = json!({
        "page": page,
        "text_layer": text_layer,
        "report_collection_date": report_collection_date,
        "printed_loinc_names": printed_loinc_names,
        "catalog_loinc_names": catalog_loinc_names,
        "source_qualified_aliases": crate::laboratory_names::SOURCE_QUALIFIED_ALIASES,
        "laboratory_metric_catalog": laboratory_metric_catalog,
        "browse_metric_index": catalog.browse_metric_index,
    });
    Ok(json!([
        {"role":"system","content":include_str!("prompts/laboratory.txt")},
        {"role":"user","content":[{"type":"text","text":evidence.to_string()},{"type":"image_url","image_url":{"url":format!("data:image/png;base64,{}",base64::engine::general_purpose::STANDARD.encode(png.into_inner()))}}]}
    ]))
}

pub fn wrap_completion(
    reply: Completion,
    model: &str,
    page: i64,
    catalog_metrics: &[CustomMetric],
) -> Result<Completion, AppError> {
    let parsed = serde_json::from_str::<serde_json::Value>(&reply.response_body).ok();
    let choice = parsed.as_ref().and_then(|v| v["choices"].get(0));
    let content = choice.and_then(|v| v["message"]["content"].as_str());
    let mut envelope = json!({"model":model,"engine":"strata","prompt_version":"laboratory-page-v22","content":content,"raw_response_body":reply.response_body});
    let mut status = reply.status_code;
    if (200..300).contains(&status) {
        let rows = content.and_then(ModelPage::parse);
        let valid = choice.is_some_and(|c| c["finish_reason"] == "stop")
            && rows.as_ref().is_some_and(|p| {
                p.observations.len() <= 2000
                    && p.observations.iter().all(|o| {
                        o.metric_id.is_none()
                            && o.unmapped_choice.is_none()
                            && o.source.page == page
                    })
                    && p.report_title.as_ref().is_none_or(|s| s.len() <= 256)
                    && p.source_name.as_ref().is_none_or(|s| s.len() <= 160)
                    && p.warnings.iter().all(|s| s.len() <= 4096)
            });
        if valid {
            let mut rows = rows.expect("validated structured response");
            for observation in &mut rows.observations {
                observation.ocr_section_heading =
                    observation.ocr_section_heading.take().filter(|heading| {
                        !heading.trim().is_empty()
                            && heading.len() <= 160
                            && !heading.chars().any(char::is_control)
                    });
                observation.ocr_metric_candidate = observation
                    .ocr_metric_candidate
                    .take()
                    .filter(|id| crate::laboratory::known_ocr_metric(id, catalog_metrics));
                let inferred = crate::laboratory::resolved_metric(observation);
                let conflict = if crate::laboratory::test_code_conflicts(&observation.raw_name) {
                    Some("The printed test name and LOINC code identify different measurements.")
                } else if inferred.is_none()
                    && observation
                        .ocr_metric_candidate
                        .as_deref()
                        .is_some_and(|id| {
                            crate::laboratory::incompatible_unitless_candidate(
                                id,
                                observation.raw_unit.as_deref(),
                                catalog_metrics,
                            )
                        })
                {
                    Some("The printed unit is incompatible with the unitless catalog metric.")
                } else {
                    None
                };
                if let Some(message) = conflict {
                    observation.ocr_metric_candidate = None;
                    observation.ocr_new_metric = None;
                    if observation
                        .ocr_mapping_note
                        .as_deref()
                        .is_none_or(|note| note.trim().is_empty())
                    {
                        observation.ocr_mapping_note = Some(message.into());
                    }
                } else {
                    if let Some(id) = inferred {
                        observation.ocr_metric_candidate = Some(id.to_owned());
                    }
                    if observation.ocr_metric_candidate.is_some()
                        || observation
                            .ocr_new_metric
                            .as_ref()
                            .is_some_and(|proposal| !proposal.is_valid())
                    {
                        observation.ocr_new_metric = None;
                    }
                }
                if let Some(proposal) = observation.ocr_normalization.take()
                    && crate::laboratory::normalized(observation).is_some_and(|(value, unit)| {
                        use std::str::FromStr;
                        unit == proposal.unit
                            && rust_decimal::Decimal::from_str(&proposal.result).is_ok_and(
                                |proposed| rust_decimal::Decimal::from_str(&value) == Ok(proposed),
                            )
                    })
                {
                    observation.ocr_normalization = Some(proposal);
                }
            }
            let warnings = crate::laboratory::subcount_warnings(rows.observations.iter());
            for (observation, warning) in rows.observations.iter_mut().zip(warnings) {
                if let Some(warning) = warning {
                    observation.ocr_normalization = None;
                    observation.ocr_mapping_note = Some(warning);
                }
            }
            envelope["report_title"] = json!(rows.report_title);
            envelope["source_name"] = json!(rows.source_name);
            envelope["observations"] = serde_json::to_value(rows.observations)?;
            envelope["warnings"] = json!(rows.warnings);
        } else {
            status = 422;
            envelope["error"] = json!({"code":"invalid_structured_output"});
        }
    } else {
        envelope["error"] = json!({"code":"inference_service_failed"});
    }
    Ok(Completion {
        response_body: envelope.to_string(),
        status_code: status,
    })
}

/// Validate the model's original rows, including rows rejected by typed decoding.
/// Never substitute a report date or manufacture a missing result.
pub fn validation_issues(
    content: &str,
    page: i64,
    has_report_collection_date: bool,
) -> Vec<serde_json::Value> {
    let text = content.trim();
    let text = text
        .strip_prefix("```json")
        .or_else(|| text.strip_prefix("```"))
        .and_then(|text| text.trim().strip_suffix("```"))
        .unwrap_or(text)
        .trim();
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return Vec::new();
    };
    let Some(items) = value
        .as_array()
        .or_else(|| value["observations"].as_array())
    else {
        return Vec::new();
    };
    let mut issues = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let name = item["raw_name"].as_str().unwrap_or("").trim();
        if crate::laboratory::is_administrative_name(name) {
            continue;
        }
        for (field, key) in [
            ("name", "raw_name"),
            ("collection_date", "sampled_at"),
            ("value", "raw_result"),
        ] {
            let text = item[key]
                .as_str()
                .map(str::trim)
                .filter(|text| !text.is_empty());
            let reason = match text {
                None if field == "collection_date" && has_report_collection_date => None,
                None => Some("missing"),
                Some(date)
                    if field == "collection_date" && crate::reports::parse_time(date).is_err() =>
                {
                    Some("invalid_date")
                }
                _ => None,
            };
            if let Some(reason) = reason {
                issues.push(json!({"page":page,"item_index":index+1,"item_name":name,"field":field,"reason":reason}));
            }
        }
    }
    issues
}
