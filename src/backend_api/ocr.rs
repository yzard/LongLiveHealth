use crate::{
    config::OcrConfig, error::AppError, provider::Completion, reports::ObservationPayload,
};
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

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

pub async fn extract(
    cpu: &crate::execution::CpuExecutor,
    config: &OcrConfig,
    page: i64,
    image_url: String,
    text_layer: Option<crate::documents::TextLayer>,
) -> Result<Completion, AppError> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(config.timeout_seconds))
        .build()
        .map_err(|_| AppError::Internal)?;
    let request_body = cpu
        .run_background(move || {
            Ok(serde_json::to_vec(
                &json!({"page":page,"image_url":image_url,"text_layer":text_layer}),
            )?)
        })
        .await?;
    let response = client
        .post(format!(
            "{}/api/v1/documents/extract",
            config.url.trim_end_matches('/')
        ))
        .bearer_auth(&config.api_key)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(request_body)
        .send()
        .await
        .map_err(|_| AppError::ProviderUnavailable)?;
    crate::provider::read_completion(response).await
}

pub async fn probe(config: &OcrConfig) -> Result<(), AppError> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| AppError::Internal)?;
    let result = client
        .get(format!("{}/health", config.url.trim_end_matches('/')))
        .send()
        .await
        .map_err(|_| AppError::ProviderUnavailable)?;
    if !result.status().is_success() {
        return Err(AppError::ProviderUnavailable);
    }
    Ok(())
}

/// Validate the model's original rows, including rows rejected by typed decoding.
/// Never substitute a report date or manufacture a missing result.
pub fn validation_issues(content: &str, page: i64) -> Vec<serde_json::Value> {
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
    let Some(items) = value["observations"].as_array() else {
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
