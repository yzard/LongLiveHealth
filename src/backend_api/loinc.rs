use std::{
    collections::{BTreeSet, HashMap},
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use crate::{
    authentication::CurrentUser, error::AppError, interface_language::Language,
    laboratory::valid_test_code,
};
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
    terms: HashMap<String, TermDefinition>,
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
    #[serde(rename = "COMPONENT")]
    component: &'a str,
    #[serde(rename = "PROPERTY")]
    property: &'a str,
    #[serde(rename = "TIME_ASPCT")]
    time: &'a str,
    #[serde(rename = "SYSTEM")]
    specimen: &'a str,
    #[serde(rename = "SCALE_TYP")]
    scale: &'a str,
    #[serde(rename = "METHOD_TYP")]
    method: &'a str,
}

struct TermDefinition {
    name: String,
    axes: [String; 5],
    method: String,
}

impl TermDefinition {
    fn differs_only_in_method(&self, other: &Self) -> bool {
        self.method != other.method
            && self.axes == other.axes
            && self.axes.iter().all(|axis| !axis.is_empty())
    }

    fn difference_labels(&self, other: &Self, language: Language) -> Vec<&'static str> {
        let labels = match language {
            Language::English => [
                "measured component",
                "measurement property",
                "time",
                "specimen",
                "result type",
            ],
            Language::Chinese => ["检测对象", "测量属性", "时间条件", "标本", "结果类型"],
        };
        self.axes
            .iter()
            .zip(&other.axes)
            .zip(labels)
            .filter_map(|((left, right), label)| (left != right).then_some(label))
            .collect()
    }
}

impl Reference {
    fn load(path: &Path) -> Result<Self, &'static str> {
        let file = File::open(path).map_err(|_| "Cannot open the official LOINC release ZIP")?;
        let mut zip = zip::ZipArchive::new(file).map_err(|_| "Invalid LOINC release ZIP")?;
        let (version, terms) = Self::read_terms(
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
            if source == target || !terms.contains_key(source) || !terms.contains_key(target) {
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
            terms,
            replacements,
        })
    }

    fn read_terms(
        input: impl Read,
    ) -> Result<(String, HashMap<String, TermDefinition>), &'static str> {
        let mut csv = csv::Reader::from_reader(input);
        let headers = csv
            .headers()
            .map_err(|_| "Invalid Loinc.csv header")?
            .clone();
        let mut terms = HashMap::new();
        let mut latest = None;
        for row in csv.records() {
            let row = row.map_err(|_| "Invalid row in Loinc.csv")?;
            let term: Term<'_> = row
                .deserialize(Some(&headers))
                .map_err(|_| "Invalid LOINC term columns")?;
            if !valid_test_code(term.code) || term.name.trim().is_empty() {
                return Err("Loinc.csv contains an invalid code or term name");
            }
            let definition = TermDefinition {
                name: term.name.to_owned(),
                axes: [
                    term.component,
                    term.property,
                    term.time,
                    term.specimen,
                    term.scale,
                ]
                .map(|axis| axis.trim().to_owned()),
                method: term.method.trim().to_owned(),
            };
            if terms.insert(term.code.to_owned(), definition).is_some() {
                return Err("Loinc.csv contains a duplicate LOINC code");
            }
            // A few historical rows have pre-release tags (for example 1.0ma).
            let version = term.version.split_once('.').and_then(|(major, minor)| {
                Some((major.parse::<u16>().ok()?, minor.parse::<u16>().ok()?))
            });
            latest = latest.max(version);
        }
        let (major, minor) = latest.ok_or("Loinc.csv contains no terms")?;
        Ok((format!("{major}.{minor:02}"), terms))
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
        reference.terms.len(),
        reference.replacements.values().map(BTreeSet::len).sum(),
    ))
}

pub(crate) fn long_common_name(code: &str) -> Option<&'static str> {
    reference()
        .ok()?
        .terms
        .get(code)
        .map(|term| term.name.as_str())
}

#[derive(Debug)]
pub struct MappingFailure {
    incoming: String,
    existing: String,
    version: String,
    ambiguous: bool,
    incompatible_units: Option<(String, String)>,
}

pub(crate) fn validate_association(
    incoming: &str,
    existing: &BTreeSet<&str>,
    metric_id: &str,
    standard_unit: Option<&str>,
    payload: &crate::reports::ObservationPayload,
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
    let matching_method_variants = direct.is_none()
        && reference.terms.get(incoming).is_some_and(|report| {
            existing.iter().any(|code| {
                reference
                    .terms
                    .get(*code)
                    .is_some_and(|database| report.differs_only_in_method(database))
            })
        });
    let incompatible_units = if matching_method_variants {
        if crate::laboratory::association_units_compatible(metric_id, standard_unit, payload) {
            return Ok(());
        }
        Some((
            payload.raw_unit.clone().unwrap_or_default(),
            standard_unit.unwrap_or_default().to_owned(),
        ))
    } else {
        None
    };
    Err(AppError::LoincMapping(Box::new(MappingFailure {
        incoming: incoming.to_owned(),
        existing: existing_codes(),
        version: reference.version.clone(),
        ambiguous: direct == Some(false),
        incompatible_units,
    })))
}

impl MappingFailure {
    pub(crate) fn message(&self, language: Language) -> String {
        let Self {
            incoming,
            existing,
            version,
            ambiguous,
            incompatible_units,
        } = self;
        let reason = if *ambiguous {
            language.message("The matching code has multiple official replacements.")
        } else {
            language.message("There is no direct replacement between these codes.")
        };
        let details = comparison_details(incoming, existing, language);
        let units = incompatible_units.as_ref().map_or_else(String::new, |(reported, standard)| {
            let reported = field_value(reported, language);
            let standard = field_value(standard, language);
            match language {
                Language::English => format!(" Units are incompatible or missing. Report unit {reported}; database unit {standard}."),
                Language::Chinese => format!(" 单位缺失或不兼容。报告单位为 {reported}；数据库单位为 {standard}。"),
            }
        });
        match language {
            Language::English => format!(
                "Cannot add report LOINC {incoming} to a metric with database LOINC {existing}. {reason} Source: LOINC {version} MapTo.csv and Loinc.csv. {details}{units} Choose the metric for the printed code or review the codes."
            ),
            Language::Chinese => format!(
                "无法将报告 LOINC {incoming} 添加到数据库 LOINC 为 {existing} 的指标。{reason}来源为 LOINC {version} 的 MapTo.csv 和 Loinc.csv。{details}{units} 请选择报告代码对应的指标，或核对代码。"
            ),
        }
    }
}

fn field_value(value: &str, language: Language) -> &str {
    if value.is_empty() {
        language.message("Not specified")
    } else {
        value
    }
}

fn term_description(code: &str, term: Option<&TermDefinition>, language: Language) -> String {
    let method = term.map_or("", |term| term.method.as_str());
    let scale = term.map_or("", |term| term.axes[4].as_str());
    let scale = match scale {
        "Qn" => language.message("Quantitative"),
        "SemiQn" => language.message("Semiquantitative"),
        "Ord" => language.message("Ordinal"),
        "OrdQn" => language.message("Ordinal or quantitative"),
        "Nom" => language.message("Nominal"),
        _ => field_value(scale, language),
    };
    let method = field_value(method, language);
    match language {
        Language::English => format!("{code} (method: {method}; result type: {scale})"),
        Language::Chinese => format!("{code}（检测方法：{method}；结果类型：{scale}）"),
    }
}

pub(crate) fn comparison_details(incoming: &str, existing: &str, language: Language) -> String {
    let Ok(reference) = reference() else {
        return String::new();
    };
    let report_term = reference.terms.get(incoming);
    let report = term_description(incoming, report_term, language);
    let targets = existing
        .split(", ")
        .filter(|code| !code.is_empty())
        .map(|code| {
            let term = reference.terms.get(code);
            let description = term_description(code, term, language);
            let differences = report_term
                .zip(term)
                .map(|(report, database)| report.difference_labels(database, language));
            if let Some(differences) = differences.filter(|fields| !fields.is_empty()) {
                let label = language.message("Different fields");
                return format!("{description} {label}: {}.", differences.join(", "));
            }
            format!("{description}.")
        })
        .collect::<Vec<_>>()
        .join(" ");
    if targets.is_empty() {
        return match language {
            Language::English => format!("Report {report}."),
            Language::Chinese => format!("报告 {report}。"),
        };
    }
    match language {
        Language::English => format!("Report {report}. Database {targets}"),
        Language::Chinese => format!("报告 {report}。数据库 {targets}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacement_lookup_distinguishes_direct_ambiguous_and_unrelated_pairs() {
        let reference = Reference {
            version: "9.17".into(),
            terms: HashMap::new(),
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
