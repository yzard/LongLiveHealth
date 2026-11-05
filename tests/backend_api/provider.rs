use serde_json::{Value, json};

pub fn completion(page: Value) -> Value {
    let mut observations = page["observations"].as_array().cloned().unwrap_or_default();
    for observation in &mut observations {
        observation["metric_id"] = Value::Null;
    }
    let page = json!({"report_title":page["report_title"],"source_name":page["source_name"],"observations":observations,"warnings":page["warnings"]});
    json!({"choices":[{"message":{"content":page.to_string()},"finish_reason":"stop"}]})
}
pub fn request_page(request: &Value) -> i64 {
    let evidence: Value = serde_json::from_str(
        request["messages"][1]["content"][0]["text"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    evidence["page"].as_i64().unwrap()
}
pub async fn mock() -> (String, tokio::task::JoinHandle<()>) {
    async fn respond(
        headers: axum::http::HeaderMap,
        axum::Json(request): axum::Json<Value>,
    ) -> axum::Json<Value> {
        assert_eq!(
            headers["authorization"],
            "Bearer synthetic-ocr-service-key-123456789"
        );
        if request["messages"][0]["content"]
            .as_str()
            .is_some_and(|prompt| prompt.contains("Review confirmed lipid"))
        {
            let input: Value =
                serde_json::from_str(request["messages"][1]["content"].as_str().unwrap()).unwrap();
            let output=json!({"summary":"Discuss the observed lipid trend.","findings":[{"topic":"lipid_risk","title":"Lipid trend","hypothesis":"A risk factor to discuss, not a diagnosis.","observation_ids":[input["observations"][0]["observation_id"]],"evidence_source_ids":["medlineplus-cholesterol"],"other_explanations":["Measurement context"],"missing_information":["Family history"],"questions_for_clinician":["How should this trend be evaluated?"]}]}).to_string();
            return axum::Json(
                json!({"choices":[{"message":{"content":output},"finish_reason":"stop"}]}),
            );
        }
        assert!(
            request["messages"][1]["content"][1]["image_url"]["url"]
                .as_str()
                .unwrap()
                .starts_with("data:image/png;base64,")
        );
        let mut observation = crate::observation_payload();
        observation.metric_id = None;
        observation.source.page = request_page(&request);
        axum::Json(completion(
            json!({"report_title":"Comprehensive laboratory report","source_name":"Function Health","observations":[observation],"warnings":[]}),
        ))
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        axum::serve(
            listener,
            axum::Router::new().route("/v1/chat/completions", axum::routing::post(respond)),
        )
        .await
        .unwrap();
    });
    (format!("http://{address}/v1"), task)
}

#[test]
fn structured_response_requires_valid_complete_json() {
    assert_eq!(
        longlivehealth::provider::parse_json::<Value>("```json\n{\"ok\":true}\n```").unwrap()["ok"],
        true
    );
    assert!(longlivehealth::provider::parse_json::<Value>("```json {} ").is_err());
    assert!(longlivehealth::provider::parse_json::<Value>("not json").is_err());
}

#[tokio::test]
async fn temporary_provider_failure_retries_but_auth_failure_does_not() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    for code in [429, 401] {
        let attempts = Arc::new(AtomicUsize::new(0));
        let counter = attempts.clone();
        let router = axum::Router::new().route(
            "/v1/chat/completions",
            axum::routing::post(move |headers: axum::http::HeaderMap| {
                let counter = counter.clone();
                async move {
                    assert_eq!(headers["authorization"], "Bearer synthetic-analysis-key");
                    if counter.fetch_add(1, Ordering::SeqCst) == 0 {
                        (
                            axum::http::StatusCode::from_u16(code).unwrap(),
                            axum::Json(json!({})),
                        )
                    } else {
                        (
                            axum::http::StatusCode::OK,
                            axum::Json(json!({"choices":[{"message":{"content":"ready"}}]})),
                        )
                    }
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        let mut config: longlivehealth::config::Config =
            toml::from_str(longlivehealth::config::TEMPLATE).unwrap();
        config.analysis_enabled = true;
        config.llm.api_key = "synthetic-analysis-key".into();
        config.llm.base_url = format!("http://{address}/v1");
        let result = longlivehealth::provider::probe(&config.llm).await;
        assert_eq!(result.is_ok(), code == 429);
        assert_eq!(
            attempts.load(Ordering::SeqCst),
            if code == 429 { 2 } else { 1 }
        );
        task.abort();
    }
}
