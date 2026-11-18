use std::{
    collections::HashMap,
    io::{Cursor, Read},
    sync::LazyLock,
};

use crate::{authentication::CurrentUser, error::AppError};
use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub(crate) struct NameRequest {
    code: String,
}

#[derive(Serialize)]
pub(crate) struct NameResult {
    official_name: Option<&'static str>,
}

pub(crate) async fn get(
    _: CurrentUser,
    Json(request): Json<NameRequest>,
) -> Result<Json<NameResult>, AppError> {
    if !crate::laboratory::valid_test_code(&request.code) {
        return Err(AppError::Invalid("Invalid LOINC code"));
    }
    Ok(Json(NameResult {
        official_name: long_common_name(&request.code),
    }))
}

// LOINC 2.83 LONG_COMMON_NAME records from the NLM Clinical Table Search Service.
// See the LOINC notice in the project license.
static LONG_COMMON_NAMES: LazyLock<HashMap<&str, &str>> = LazyLock::new(|| {
    static DATA: LazyLock<String> = LazyLock::new(|| {
        let bytes = include_bytes!("loinc_names.zip");
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("bundled LOINC archive");
        let mut names = String::new();
        archive
            .by_index(0)
            .expect("bundled LOINC names")
            .read_to_string(&mut names)
            .expect("UTF-8 LOINC names");
        names
    });
    DATA.lines()
        .map(|row| {
            row.split_once('\t')
                .expect("LOINC code and long common name")
        })
        .collect()
});

pub(crate) fn long_common_name(code: &str) -> Option<&'static str> {
    LONG_COMMON_NAMES.get(code).copied()
}
