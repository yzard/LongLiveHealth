use longlivehealth::{ocr, provider::Completion};
use serde_json::{Value, json};

#[test]
fn shared_inference_archives_original_and_rejects_truncation_or_untrusted_ids() {
    let mut observation = crate::observation_payload();
    observation.metric_id = None;
    let page = json!({"report_title":"Blood test","source_name":"Synthetic Lab","observations":[observation],"warnings":[]});
    let reply = |value: Value, finish: &str| {
        json!({"choices":[{"message":{"content":value.to_string()},"finish_reason":finish}]})
            .to_string()
    };
    let body = reply(page.clone(), "stop");
    let result = ocr::wrap_completion(
        Completion {
            response_body: body.clone(),
            status_code: 200,
        },
        "shared-model",
        1,
    )
    .unwrap();
    assert_eq!(result.status_code, 200);
    let envelope: Value = serde_json::from_str(&result.response_body).unwrap();
    assert_eq!(envelope["raw_response_body"], body);
    assert_eq!(envelope["engine"], "strata");
    assert_eq!(envelope["source_name"], "Synthetic Lab");
    for (mut value, finish) in [(page.clone(), "length"), (page.clone(), "stop")] {
        if finish == "stop" {
            value["observations"][0]["metric_id"] = json!("invented-analyte");
        }
        let raw = reply(value, finish);
        let result = ocr::wrap_completion(
            Completion {
                response_body: raw.clone(),
                status_code: 200,
            },
            "shared-model",
            1,
        )
        .unwrap();
        assert_eq!(result.status_code, 422);
        assert_eq!(
            serde_json::from_str::<Value>(&result.response_body).unwrap()["raw_response_body"],
            raw
        );
    }
    let wrong_page = ocr::wrap_completion(
        Completion {
            response_body: reply(page, "stop"),
            status_code: 200,
        },
        "shared-model",
        2,
    )
    .unwrap();
    assert_eq!(wrong_page.status_code, 422);
}

#[test]
fn page_preparation_is_bounded_inline_and_preserves_untrusted_text_evidence() {
    use base64::Engine;
    let mut image = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(400, 400)
        .write_to(&mut image, image::ImageFormat::Png)
        .unwrap();
    let url = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(image.into_inner())
    );
    let evidence = longlivehealth::documents::TextLayer {
        status: "empty".into(),
        text: String::new(),
        words: vec![],
    };
    let messages = ocr::prepare_messages(3, &url, Some(evidence), 65536, 40000000).unwrap();
    assert!(
        messages[0]["content"]
            .as_str()
            .unwrap()
            .contains("Never convert units")
    );
    let data: Value =
        serde_json::from_str(messages[1]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(data["page"], 3);
    assert_eq!(data["text_layer"]["status"], "empty");
    let output = messages[1]["content"][1]["image_url"]["url"]
        .as_str()
        .unwrap()
        .split_once(',')
        .unwrap()
        .1;
    let image = image::load_from_memory(
        &base64::engine::general_purpose::STANDARD
            .decode(output)
            .unwrap(),
    )
    .unwrap();
    assert!(u64::from(image.width()) * u64::from(image.height()) <= 65536);
    assert!(
        ocr::prepare_messages(
            1,
            "http://private.example/report.png",
            None,
            65536,
            40000000
        )
        .is_err()
    );
    assert!(ocr::prepare_messages(1, &url, None, 65536, 1).is_err());
}
