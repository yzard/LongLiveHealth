use std::{
    collections::{BTreeSet, HashMap},
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use crate::{authentication::CurrentUser, error::AppError, laboratory::valid_test_code};
use axum::Json;
use serde::{Deserialize, Serialize};

const IMAGE_ARCHIVE: &str = "/app/loinc/Loinc.zip";

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
    if !valid_test_code(&request.code) {
        return Err(AppError::Invalid("Invalid LOINC code"));
    }
    Ok(Json(NameResult {
        official_name: long_common_name(&request.code),
    }))
}

struct Reference {
    version: String,
    names: HashMap<String, String>,
    replacements: HashMap<String, BTreeSet<String>>,
}

#[derive(Deserialize)]
struct Term<'a> {
    #[serde(rename = "LOINC_NUM")]
    code: &'a str,
    #[serde(rename = "LONG_COMMON_NAME")]
    name: &'a str,
    #[serde(rename = "VersionLastChanged")]
    version: &'a str,
}

impl Reference {
    fn load(path: &Path) -> Result<Self, &'static str> {
        let file = File::open(path).map_err(|_| "Cannot open the official LOINC release ZIP")?;
        let mut zip = zip::ZipArchive::new(file).map_err(|_| "Invalid LOINC release ZIP")?;
        let (version, names) = Self::read_terms(
            zip.by_name("LoincTable/Loinc.csv")
                .map_err(|_| "LOINC release ZIP is missing LoincTable/Loinc.csv")?,
        )?;
        let mut csv = csv::Reader::from_reader(
            zip.by_name("LoincTable/MapTo.csv")
                .map_err(|_| "LOINC release ZIP is missing LoincTable/MapTo.csv")?,
        );
        if !csv
            .headers()
            .is_ok_and(|header| header.iter().eq(["LOINC", "MAP_TO", "COMMENT"]))
        {
            return Err("MapTo.csv must have the LOINC, MAP_TO and COMMENT columns");
        }
        let mut replacements = HashMap::<String, BTreeSet<String>>::new();
        for row in csv.records() {
            let row = row.map_err(|_| "Invalid row in MapTo.csv")?;
            let source = &row[0];
            let target = &row[1];
            if source == target || !names.contains_key(source) || !names.contains_key(target) {
                return Err("MapTo.csv contains an invalid or unknown LOINC replacement");
            }
            if !replacements
                .entry(source.to_owned())
                .or_default()
                .insert(target.to_owned())
            {
                return Err("MapTo.csv contains a duplicate replacement");
            }
        }
        if replacements.is_empty() {
            return Err("MapTo.csv contains no replacement rows");
        }
        Ok(Self {
            version,
            names,
            replacements,
        })
    }

    fn read_terms(input: impl Read) -> Result<(String, HashMap<String, String>), &'static str> {
        let mut csv = csv::Reader::from_reader(input);
        let headers = csv
            .headers()
            .map_err(|_| "Invalid Loinc.csv header")?
            .clone();
        let mut names = HashMap::new();
        let mut latest = None;
        for row in csv.records() {
            let row = row.map_err(|_| "Invalid row in Loinc.csv")?;
            let term: Term<'_> = row
                .deserialize(Some(&headers))
                .map_err(|_| "Invalid LOINC term columns")?;
            if !valid_test_code(term.code) || term.name.trim().is_empty() {
                return Err("Loinc.csv contains an invalid code or term name");
            }
            if names
                .insert(term.code.to_owned(), term.name.to_owned())
                .is_some()
            {
                return Err("Loinc.csv contains a duplicate LOINC code");
            }
            // A few historical rows have pre-release tags (for example 1.0ma).
            let version = term.version.split_once('.').and_then(|(major, minor)| {
                Some((major.parse::<u16>().ok()?, minor.parse::<u16>().ok()?))
            });
            latest = latest.max(version);
        }
        let (major, minor) = latest.ok_or("Loinc.csv contains no terms")?;
        Ok((format!("{major}.{minor:02}"), names))
    }

    fn check_replacement(&self, incoming: &str, existing: &BTreeSet<&str>) -> Option<bool> {
        // The file is directional. Check each pair in both directions, and
        // allow a direct mapping only when its official source has one target.
        let direction = |source: &str, target: &str| {
            let targets = self.replacements.get(source)?;
            targets.contains(target).then_some(targets.len() == 1)
        };
        existing
            .iter()
            .flat_map(|&code| [direction(incoming, code), direction(code, incoming)])
            .flatten()
            .max()
    }
}

fn reference() -> Result<&'static Reference, &'static str> {
    static REFERENCE: OnceLock<Result<Reference, &'static str>> = OnceLock::new();
    REFERENCE
        .get_or_init(|| {
            let path = std::env::var_os("LONGLIVEHEALTH_LOINC_ARCHIVE")
                .map_or_else(|| PathBuf::from(IMAGE_ARCHIVE), PathBuf::from);
            Reference::load(&path)
        })
        .as_ref()
        .map_err(|&message| message)
}

pub(crate) fn check_archive() -> Result<(&'static str, usize, usize), AppError> {
    let reference = reference().map_err(AppError::Invalid)?;
    Ok((
        &reference.version,
        reference.names.len(),
        reference.replacements.values().map(BTreeSet::len).sum(),
    ))
}

pub(crate) fn long_common_name(code: &str) -> Option<&'static str> {
    reference().ok()?.names.get(code).map(String::as_str)
}

pub(crate) fn validate_replacement(
    incoming: &str,
    existing: &BTreeSet<&str>,
) -> Result<(), AppError> {
    if existing.is_empty() || existing.contains(incoming) {
        return Ok(());
    }
    let existing_codes = || existing.iter().copied().collect::<Vec<_>>().join(", ");
    let reference = reference().map_err(|_| AppError::LoincMapUnavailable {
        incoming: incoming.to_owned(),
        existing: existing_codes(),
    })?;
    let direct = reference.check_replacement(incoming, existing);
    if direct == Some(true) {
        return Ok(());
    }
    Err(AppError::LoincMapping {
        incoming: incoming.to_owned(),
        existing: existing_codes(),
        version: reference.version.clone(),
        ambiguous: direct == Some(false),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacement_lookup_distinguishes_direct_ambiguous_and_unrelated_pairs() {
        let reference = Reference {
            version: "9.17".into(),
            names: HashMap::new(),
            replacements: HashMap::from([
                ("100-1".into(), BTreeSet::from(["200-1".into()])),
                ("200-1".into(), BTreeSet::from(["300-1".into()])),
                (
                    "400-1".into(),
                    BTreeSet::from(["100-1".into(), "200-1".into()]),
                ),
            ]),
        };
        let existing = BTreeSet::from(["100-1"]);
        assert_eq!(reference.check_replacement("200-1", &existing), Some(true));
        assert_eq!(reference.check_replacement("300-1", &existing), None);
        assert_eq!(reference.check_replacement("400-1", &existing), Some(false));
        assert_eq!(
            reference.check_replacement("100-1", &BTreeSet::from(["200-1"])),
            Some(true)
        );
    }
}
