use axum::{extract::Request, http::header, middleware::Next, response::Response};
use std::{collections::HashMap, sync::OnceLock};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Language {
    English,
    Chinese,
}

tokio::task_local! {
    static REQUEST_LANGUAGE: Language;
}

impl Language {
    fn negotiate(header: &str) -> Self {
        header
            .split(',')
            .filter_map(|item| {
                let mut fields = item.trim().split(';');
                let base = fields.next()?.trim().split(['-', '_']).next()?;
                let language = match base.to_ascii_lowercase().as_str() {
                    "en" | "*" => Self::English,
                    "zh" => Self::Chinese,
                    _ => return None,
                };
                let quality = fields
                    .find_map(|field| field.trim().strip_prefix("q="))
                    .map_or(Some(1.0_f32), |value| value.parse().ok())?;
                (quality > 0.0 && quality <= 1.0).then_some((language, quality))
            })
            .fold(None, |best, candidate| match best {
                Some((_, quality)) if quality >= candidate.1 => best,
                _ => Some(candidate),
            })
            .map_or(Self::English, |(language, _)| language)
    }

    pub(crate) fn current() -> Self {
        REQUEST_LANGUAGE
            .try_with(|value| *value)
            .unwrap_or(Self::English)
    }

    pub(crate) fn tag(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Chinese => "zh-Hans",
        }
    }

    pub(crate) fn message(self, english: &str) -> &str {
        if self == Self::English {
            return english;
        }
        static CHINESE: OnceLock<HashMap<String, String>> = OnceLock::new();
        CHINESE
            .get_or_init(|| {
                serde_json::from_str(include_str!(
                    "../shared/Core/Resources/InterfaceChinese.json"
                ))
                .expect("interface translations are validated at build time")
            })
            .get(english)
            .map(String::as_str)
            .unwrap_or(english)
    }
}

/// Scope the language to this request. Stored records and log messages are never translated here.
pub(crate) async fn with_request_language(request: Request, next: Next) -> Response {
    let language = request
        .headers()
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|value| value.to_str().ok())
        .map_or(Language::English, Language::negotiate);
    REQUEST_LANGUAGE.scope(language, next.run(request)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chooses_by_quality_and_preserves_order_on_ties() {
        assert_eq!(Language::negotiate("en, zh-CN"), Language::English);
        assert_eq!(Language::negotiate("en;q=0.4, zh-Hans"), Language::Chinese);
        assert_eq!(Language::negotiate("zh;q=0, en"), Language::English);
        assert_eq!(Language::negotiate("fr, zh-TW;q=0.5"), Language::Chinese);
        assert_eq!(Language::negotiate("zh;q=bad"), Language::English);
    }
}
