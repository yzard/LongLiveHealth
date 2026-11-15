//! Deterministic, analyte-specific derivation. Printed values are never rewritten.
use crate::reports::ObservationPayload;
use rust_decimal::Decimal;
use serde::Serialize;
use std::str::FromStr;

pub const CONVERSION_VERSION: &str = "lab-units-v11";
pub const CANONICAL_UNITS: &[&str] = &[
    "mg/dL",
    "mg/L",
    "g/L",
    "g/dL",
    "mmol/L",
    "µmol/L",
    "ng/mL",
    "µg/L",
    "µg/mL",
    "ng/L",
    "%",
    "mmol/mol",
    "L",
    "mL",
    "µL",
    "nL",
    "pL",
    "fL",
    "U/L",
    "10^9/L",
    "10^12/L",
    "cells/L",
    "cells/mL",
    "cells/µL",
    "pg",
    "pg/mL",
    "ng/dL",
    "µg/dL",
    "nmol/L",
    "mIU/L",
    "mIU/mL",
    "IU/mL",
    "U/mL",
    "Angstrom",
    "% by wt",
    "/100 WBC's",
    "1",
    "mL/min/1.73m2",
];

#[derive(Serialize)]
pub struct MetricDefinition {
    pub metric_id: &'static str,
    pub name: &'static str,
    pub standard_unit: &'static str,
    pub accepted_units: Vec<&'static str>,
    pub conversion_version: &'static str,
}
pub fn metrics() -> Vec<MetricDefinition> {
    [
        ("total_cholesterol", "Total cholesterol", "mg/dL"),
        ("ldl_cholesterol", "LDL cholesterol", "mg/dL"),
        ("hdl_cholesterol", "HDL cholesterol", "mg/dL"),
        ("triglycerides", "Triglycerides", "mg/dL"),
        ("apob", "Apolipoprotein B", "mg/dL"),
        ("hba1c", "HbA1c (NGSP)", "%"),
        ("glucose", "Glucose", "mg/dL"),
        ("hemoglobin", "Hemoglobin", "g/dL"),
        ("creatinine", "Creatinine", "mg/dL"),
        ("ferritin", "Ferritin", "ng/mL"),
        ("albumin", "Albumin", "g/dL"),
        ("crp", "C-reactive protein", "mg/dL"),
        ("lymphocyte_percent", "Lymphocytes %", "%"),
        ("mcv", "Mean corpuscular volume", "fL"),
        ("rdw", "Red cell distribution width (CV)", "%"),
        ("alp", "Alkaline phosphatase", "U/L"),
        ("wbc", "White blood cell count", "10^9/L"),
    ]
    .into_iter()
    .chain(
        crate::laboratory_names::EXTRA
            .iter()
            .map(|&(id, name, unit, _)| (id, name, unit)),
    )
    .map(|(metric_id, name, standard_unit)| MetricDefinition {
        metric_id,
        name,
        standard_unit,
        accepted_units: CANONICAL_UNITS
            .iter()
            .copied()
            .filter(|unit| conversion(metric_id, standard_unit, unit).is_some())
            .collect(),
        conversion_version: CONVERSION_VERSION,
    })
    .collect()
}

static DEFINITIONS: std::sync::LazyLock<
    std::collections::BTreeMap<&'static str, MetricDefinition>,
> = std::sync::LazyLock::new(|| {
    metrics()
        .into_iter()
        .map(|metric| (metric.metric_id, metric))
        .collect()
});

pub(crate) fn metric_definition(id: &str) -> Option<&'static MetricDefinition> {
    DEFINITIONS.get(id)
}

pub(crate) fn metric_aliases(id: &str) -> impl Iterator<Item = &'static str> {
    crate::laboratory_names::EXISTING_ALIASES
        .iter()
        .filter(move |(metric, _)| *metric == id)
        .flat_map(|(_, aliases)| aliases.iter().copied())
        .chain(
            crate::laboratory_names::EXTRA
                .iter()
                .filter(move |(metric, _, _, _)| *metric == id)
                .flat_map(|(_, _, _, aliases)| aliases.iter().copied()),
        )
}

pub(crate) type OcrMetricEntry<'a> = (&'a str, &'a str, &'a str, Vec<&'a str>, Vec<&'a str>);

pub(crate) fn ocr_metric_catalog<'a>(
    account_metrics: &'a [CustomMetric],
    account_codes: &'a std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
) -> Vec<OcrMetricEntry<'a>> {
    let mut catalog: std::collections::BTreeMap<_, _> = DEFINITIONS
        .values()
        .map(|metric| (metric.metric_id, (metric.name, metric.standard_unit)))
        .collect();
    for metric in account_metrics {
        catalog.insert(&metric.metric_id, (&metric.name, &metric.standard_unit));
    }
    catalog
        .into_iter()
        .map(|(id, (name, unit))| {
            let aliases = metric_aliases(id)
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            let codes = account_codes
                .get(id)
                .into_iter()
                .flatten()
                .map(String::as_str)
                .chain(
                    LOINC_METRICS
                        .iter()
                        .filter_map(|&(code, metric)| (metric == id).then_some(code)),
                )
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            (id, name, unit, aliases, codes)
        })
        .collect()
}

pub(crate) fn known_ocr_metric(id: &str, account_metrics: &[CustomMetric]) -> bool {
    DEFINITIONS.contains_key(id) || account_metrics.iter().any(|metric| metric.metric_id == id)
}

pub(crate) fn incompatible_unitless_candidate(
    id: &str,
    raw_unit: Option<&str>,
    account_metrics: &[CustomMetric],
) -> bool {
    let standard_unit = DEFINITIONS
        .get(id)
        .map(|metric| metric.standard_unit)
        .or_else(|| {
            account_metrics
                .iter()
                .find(|metric| metric.metric_id == id)
                .map(|metric| metric.standard_unit.as_str())
        });
    standard_unit == Some("1")
        && raw_unit
            .filter(|unit| !unit.trim().is_empty())
            .is_some_and(|raw| {
                canonical_unit(raw).is_none_or(|unit| conversion(id, "1", unit).is_none())
            })
}

pub(crate) fn verified_custom_candidate<'a>(
    payload: &ObservationPayload,
    custom: &'a [CustomMetric],
) -> Option<&'a str> {
    if payload
        .ocr_mapping_note
        .as_ref()
        .is_some_and(|note| !note.trim().is_empty())
    {
        return None;
    }
    let id = payload.ocr_metric_candidate.as_deref()?;
    if !id.starts_with("custom:") {
        return None;
    }
    let (name, code) = split_test_code(&payload.raw_name);
    if code.and_then(loinc_metric).is_some() {
        return None;
    }
    let unit = payload
        .raw_unit
        .as_deref()
        .map(custom_unit)
        .unwrap_or_default();
    let key = crate::laboratory_names::key(name);
    let mut exact = custom.iter().filter(|metric| {
        metric.standard_unit == unit && crate::laboratory_names::key(&metric.name) == key
    });
    let matched = exact.next()?;
    (matched.metric_id == id && exact.next().is_none()).then_some(matched.metric_id.as_str())
}

/// A numeric test code in the final parentheses is separate from the printed name.
pub fn split_test_code(name: &str) -> (&str, Option<&str>) {
    let name = name.trim();
    if let Some((label, code)) = name
        .strip_suffix(')')
        .and_then(|value| value.rsplit_once('('))
        && !label.trim().is_empty()
        && let Some((number, check_digit)) = code.trim().split_once('-')
        && (1..=7).contains(&number.len())
        && number.bytes().all(|byte| byte.is_ascii_digit())
        && check_digit.len() == 1
        && check_digit.bytes().all(|byte| byte.is_ascii_digit())
    {
        return (label.trim_end(), Some(code.trim()));
    }
    (name, None)
}

pub fn display_name<'a>(raw_name: &'a str, metric_id: Option<&str>) -> &'a str {
    metric_id
        .and_then(|id| DEFINITIONS.get(id))
        .map_or_else(|| split_test_code(raw_name).0, |metric| metric.name)
}

// Closed LOINC mapping, verified against https://loinc.org/<code>/.
// Unknown codes remain in the raw archive; labels are never guessed from display text.
pub(crate) const LOINC_METRICS: &[(&str, &str)] = &[
    ("10834-0", "globulin"),
    ("11054-4", "ldl_hdl_ratio"),
    ("11580-8", "tsh"),
    ("13457-7", "ldl_cholesterol"),
    ("13458-5", "vldl_cholesterol"),
    ("14804-9", "ldh"),
    ("1742-6", "alt"),
    ("1751-7", "albumin"),
    ("17856-6", "hba1c"),
    ("17861-6", "calcium"),
    ("18262-6", "ldl_cholesterol"),
    ("18309-5", "nrbc_per_100_wbc"),
    ("1884-6", "apob"),
    ("19123-9", "serum_magnesium"),
    ("1920-8", "ast"),
    ("1975-2", "total_bilirubin"),
    ("1988-5", "crp"),
    ("1989-3", "vitamin_d_25_oh_d3"),
    ("2028-9", "carbon_dioxide"),
    ("20454-5", "urine_protein"),
    ("2075-0", "chloride"),
    ("2085-9", "hdl_cholesterol"),
    ("2089-1", "ldl_cholesterol"),
    ("2093-3", "total_cholesterol"),
    ("2095-8", "hdl_percent_total_cholesterol"),
    ("2160-0", "creatinine"),
    ("2276-4", "ferritin"),
    ("2324-2", "ggt"),
    ("2336-6", "globulin"),
    ("2345-7", "glucose"),
    ("2498-4", "iron"),
    ("2514-8", "urine_ketones"),
    ("25428-4", "urine_glucose"),
    ("2571-8", "triglycerides"),
    ("2731-8", "intact_pth"),
    ("2777-1", "serum_phosphate"),
    ("2823-3", "potassium"),
    ("2885-2", "total_protein"),
    ("2951-2", "sodium"),
    ("3013-0", "thyroglobulin"),
    ("3016-3", "tsh"),
    ("3024-7", "free_t4"),
    ("3026-2", "total_t4"),
    ("30392-5", "nrbc_count"),
    ("3040-3", "lipase"),
    ("3084-1", "uric_acid"),
    ("3094-0", "bun"),
    ("3097-3", "bun_creatinine_ratio"),
    ("32167-9", "urine_clarity"),
    ("32623-1", "mpv"),
    ("38518-7", "immature_granulocyte_percent"),
    ("44733-4", "triglyceride_hdl_ratio"),
    ("4544-3", "hematocrit"),
    ("4548-4", "hba1c"),
    ("49054-0", "vitamin_d_25_oh_d2"),
    ("50231-0", "urine_hyaline_casts"),
    ("51584-1", "immature_granulocyte_count"),
    ("53115-2", "immature_granulocyte_count"),
    ("5767-9", "urine_appearance"),
    ("5778-6", "urine_color"),
    ("5794-3", "urine_occult_blood"),
    ("5799-2", "urine_leukocyte_esterase"),
    ("5802-4", "urine_nitrite"),
    ("58413-6", "nrbc_per_100_wbc"),
    ("5905-5", "monocyte_percent"),
    ("62292-8", "vitamin_d_25_oh_total"),
    ("6690-2", "wbc"),
    ("6768-6", "alp"),
    ("68438-1", "vitamin_d_25_oh_total"),
    ("704-7", "basophil_count"),
    ("706-2", "basophil_percent"),
    ("711-2", "eosinophil_count"),
    ("713-8", "eosinophil_percent"),
    ("71695-1", "immature_granulocyte_percent"),
    ("718-7", "hemoglobin"),
    ("731-0", "lymphocyte_count"),
    ("736-9", "lymphocyte_percent"),
    ("742-7", "monocyte_count"),
    ("751-8", "neutrophil_count"),
    ("770-8", "neutrophil_percent"),
    ("771-6", "nrbc_count"),
    ("777-3", "platelets"),
    ("785-6", "mch"),
    ("786-4", "mchc"),
    ("787-2", "mcv"),
    ("788-0", "rdw"),
    ("789-8", "rbc"),
    ("8098-6", "thyroglobulin_antibodies"),
    ("8099-4", "tpo_antibodies"),
    ("83119-8", "total_t4"),
    ("9830-1", "cholesterol_hdl_ratio"),
];

pub(crate) fn loinc_metric(code: &str) -> Option<&'static str> {
    let index = LOINC_METRICS
        .binary_search_by_key(&code, |&(code, _)| code)
        .ok()?;
    Some(LOINC_METRICS[index].1)
}

/// Explicit user mappings take precedence. Inference never modifies printed payloads.
/// Administrative fields are archive context, never patient measurements.
/// Exact field names avoid dropping real tests such as an antibody screen or specimen creatinine.
pub fn is_administrative_name(name: &str) -> bool {
    matches!(
        crate::laboratory_names::key(split_test_code(name).0).as_str(),
        "specimenexpirationdate"
            | "specimenexpirationdateofblood"
            | "specimenexpirydate"
            | "specimenexpires"
            | "expirationdate"
            | "expirydate"
            | "statusinformation"
            | "bloodbankcomment"
            | "reportstatus"
            | "orderstatus"
            | "specimenstatus"
            | "processingstatus"
            | "resultstatus"
            | "patientname"
            | "patientid"
            | "medicalrecordnumber"
            | "mrn"
            | "dateofbirth"
            | "accessionnumber"
            | "accessionid"
            | "specimenid"
            | "specimennumber"
            | "reportdate"
            | "reportingdate"
            | "collectiondate"
            | "collectiontime"
            | "specimencollectiondate"
            | "specimenreceiveddate"
            | "receiveddate"
            | "orderingprovider"
            | "orderingphysician"
            | "laboratoryaddress"
    )
}

pub fn resolved_metric(payload: &ObservationPayload) -> Option<&str> {
    if is_administrative_name(&payload.raw_name) {
        return None;
    }
    payload.metric_id.as_deref().or_else(|| {
        if payload
            .ocr_mapping_note
            .as_ref()
            .is_some_and(|note| !note.trim().is_empty())
        {
            return None;
        }
        infer_metric(&payload.raw_name, payload.raw_unit.as_deref()).or_else(|| {
            let (label, code) = split_test_code(&payload.raw_name);
            let specimen =
                crate::laboratory_names::section_specimen(payload.ocr_section_heading.as_deref()?)?;
            let qualified = infer_metric(
                &format!("{label} - {specimen}"),
                payload.raw_unit.as_deref(),
            )?;
            (code.is_none() && payload.ocr_metric_candidate.as_deref() == Some(qualified))
                .then_some(qualified)
        })
    })
}

// Previously issued clinical IDs still open the corresponding history and guide.
pub(crate) fn canonical_history_id(id: &str) -> &str {
    match id {
        "clinical:edd89ae78ca4fa80bef33fcbf1acceca239aba400eab37d16677d13d83293271" => {
            "vitamin_d_25_oh_total"
        }
        "clinical:e4c1ef2c51dac3bd5240dc82556330573bee7e98f8fceefa03c70bf38cc9bf0d" => {
            "vitamin_d_25_oh_d3"
        }
        "clinical:7ab8a8adb6b2f4ec5ec4cb6fe7c9842f952dfea139183dc8c21d15a887f1c44e" => {
            "bun_creatinine_ratio"
        }
        "clinical:cba92749dfff0342bdf57a4ac09165636d5d94eb4695b2798c0bb9499782b817" => {
            "serum_phosphate"
        }
        "clinical:599284c9549c3c631d6aee4ddabe2a400a698996e714ffe4cc69e767b6245417" => {
            "thyroglobulin"
        }
        "clinical:a335f19f51d64583e1f1f6152dc993c69bdbd1147c4d35663472d81acc74e7da" => {
            "serum_magnesium"
        }
        "clinical:e72d76d9f5efb4586c214350591df35164ed183acb1ead4ef5d79a15351775bb" => "intact_pth",
        "clinical:34b9f612dde80cf147469fed472154062c0447037e31a5c11911bf2ff3f92702" => {
            "vitamin_d_25_oh_total"
        }
        "clinical:c4984e4189742ffb054258b800419c3906005e3971d0832dddb9f0feb36853e8" => {
            "immature_granulocyte_percent"
        }
        "clinical:97c1cdd6c36b505ad71b6bbe8bac2b9a796f533334789167af0c52d4c4ffa0a6" => {
            "immature_granulocyte_percent"
        }
        "clinical:96feb2056ce4641c5589c7dcabebbb4c6c95eea22df875b679d3f5d8a76b27ab" => {
            "immature_granulocyte_count"
        }
        "clinical:4241c2d11b7ae792427a06df0264b577e9be832cec44e96ea28722efe90b683e" => {
            "immature_granulocyte_count"
        }
        "clinical:b9eea2a3f386e9c03fa55e1b9dd53f349c18f4d5b0ea10ce29617c7d2ec6ce9f" => "nrbc_count",
        "clinical:cf87113870a5059412204cf0a62f8a31e468ed871a65b456587ea90af92a4a52" => "nrbc_count",
        "clinical:ef3b2d1f3d722a6d1b7d3306571a53a84560630f527c8127befcebbd735aa509" => {
            "nrbc_per_100_wbc"
        }
        "clinical:555ca6941f1883ed87ef6c5d94efd02be40988630cd977af992cd7cdd70e1745" => {
            "nrbc_per_100_wbc"
        }
        "clinical:55124fbe978a39e61c77ee3f4cd239186a4b8ca18e7f2a5c1b59f0dca8d25bb4" => "total_t4",
        "clinical:9e8b162ad7697935375292af498020eb5f88e2c1fcd33ecdc00b6b0e330f9d38" => "total_t4",
        "clinical:a7b43dc3c8d5f7e235d51d77d681a51e7964423767703202a66600a47f822083" => "ldh",
        "clinical:0b0389fdf2077f4ccedd0f7d3a116bad2518123457e5b039bee9cea9b891e22e" => {
            "vldl_cholesterol"
        }
        "clinical:150e1bcf73eb2b02af4f4a12400b00184e19c602e9321a533812e76ee610a355" => {
            "ldl_hdl_ratio"
        }
        "clinical:f8c69c3356489bd567bd37e74055adfe26ec63693de99dc8742c463e6bfcca28" => {
            "triglyceride_hdl_ratio"
        }
        "clinical:5402913b367638561a621a92e8839c87d305408498de198468a9726488876473" => {
            "hdl_percent_total_cholesterol"
        }
        "clinical:ff6cde80d22d3f74637044c84573981e38a7f8f13e3258c000657b3855c7caf1" => {
            "cholesterol_hdl_ratio"
        }
        "clinical:90acdb43a32563d5bcf1cfc436cc7b08cd9c729a622d52c4f7a349217a4a0849" => "uric_acid",
        "clinical:a3f6426d0f2a32653bfc74abe5f3e13dc107b53e9c3933442a41b752fac87a44" => "ggt",
        "clinical:a9764002077ab758442a8b2539ec96a9f2989e4e82644777d14a92d9e2508434" => "iron",
        "clinical:9c522152e76d3760632a607854467165bffa6487b5f6704f2ed3ceb98459fd0d" => {
            "tpo_antibodies"
        }
        "clinical:723c7b05071903e5de445ebd3174b6dd48c9ef0533909188c8b0bf9755b7b89f" => "hba1c",
        "clinical:416205bd8baed739ad01aa8c9655da446f3e82199e28a706ba5784981f15adfa" => {
            "urine_ketones"
        }
        "clinical:57319cf6b1370f896c84139f6e53269381db6ae8ef787fd3a941552e8c60656d" => {
            "urine_occult_blood"
        }
        "clinical:7f7ed81d79f6c5ccdd17ad2ee569f81cf617e87cfa6d0022b61ac7385f669bde" => {
            "urine_nitrite"
        }
        "clinical:cf226eab3c008056dc22a1524a39e259254f6b42c0c4a4503bfb8bbdddd6c25e" => {
            "urine_leukocyte_esterase"
        }
        "clinical:52aed447c69feda115b0c0471bea884a6e43e2b45a04e042198698bbd8aab0be" => {
            "urine_color"
        }
        "clinical:8a5e596909d99ca2fa3a06b13de89685a660123799fff0fcb394269962644975" => {
            "urine_protein"
        }
        "clinical:e0af1aba74298311164659889f0699fbfd955aa8ad37a23a522f58dc1076bd55" => {
            "urine_glucose"
        }
        "clinical:d3bfe87fd4799952812ba33e66827d3ea23adf78c439ceaaf210b75771cd7159" => {
            "urine_hyaline_casts"
        }
        _ => id,
    }
}

pub(crate) fn resolved_report_metric<'a>(
    payload: &'a ObservationPayload,
    source_name: Option<&str>,
) -> Option<&'a str> {
    let resolved = resolved_metric(payload);
    if payload.metric_id.is_some() {
        return resolved;
    }
    let source = source_name.unwrap_or("").trim();
    let label = crate::laboratory_names::key(&payload.raw_name);
    let unit = payload.raw_unit.as_deref().and_then(canonical_unit);
    crate::laboratory_names::SOURCE_QUALIFIED_ALIASES
        .iter()
        .find_map(|&(provider, name, required_unit, base, qualified)| {
            (resolved == Some(base)
                && source.eq_ignore_ascii_case(provider)
                && label == name
                && unit == Some(required_unit))
            .then_some(qualified)
        })
        .or(resolved)
}

// These codes are used with misleading labels in some reports. Their
// LOINC properties are not equivalent to the catalog candidates below.
fn incompatible_lab_code(code: &str, metric: &str) -> bool {
    matches!(
        (code, metric),
        ("15210-8", "thyroglobulin_antibodies")
            | ("32786-6", "tpo_antibodies")
            | ("31160-5", "neutrophil_percent")
            | ("51384-6", "monocyte_percent")
    )
}

pub fn test_code_conflicts(name: &str) -> bool {
    let (label, code) = split_test_code(name);
    if let Some(code) = code
        && ALIAS_INDEX
            .get(&crate::laboratory_names::key(label))
            .is_some_and(|candidates| {
                candidates
                    .iter()
                    .any(|(metric, _)| incompatible_lab_code(code, metric))
            })
    {
        return true;
    }
    code.and_then(loinc_metric).is_some_and(|coded| {
        infer_metric(name, None) != Some(coded)
            && infer_metric(label, None).is_some_and(|named| named != coded)
    })
}

type Candidates = Vec<(&'static str, &'static str)>;
static ALIAS_INDEX: std::sync::LazyLock<std::collections::BTreeMap<String, Candidates>> =
    std::sync::LazyLock::new(|| {
        let mut index = std::collections::BTreeMap::<String, Candidates>::new();
        for metric in metrics() {
            for name in std::iter::once(metric.name).chain(metric_aliases(metric.metric_id)) {
                let candidates = index.entry(crate::laboratory_names::key(name)).or_default();
                if !candidates.contains(&(metric.metric_id, metric.standard_unit)) {
                    candidates.push((metric.metric_id, metric.standard_unit));
                }
            }
        }
        index
    });

pub fn infer_metric(name: &str, raw_unit: Option<&str>) -> Option<&'static str> {
    let (name, code) = split_test_code(name);
    let key = crate::laboratory_names::key(name);
    let named = ALIAS_INDEX.get(&key);
    let accepts_unit = |id: &str, standard: &str| {
        raw_unit
            .filter(|unit| !unit.trim().is_empty())
            .is_none_or(|raw| {
                canonical_unit(raw).is_some_and(|unit| conversion(id, standard, unit).is_some())
            })
    };
    if let Some(id) = code.and_then(loinc_metric) {
        let metric = DEFINITIONS.get(id)?;
        let name_matches = key == "test"
            || crate::laboratory_names::CODE_QUALIFIED_ALIASES.contains(&(id, key.as_str()))
            || named.is_some_and(|candidates| candidates.iter().any(|(name_id, _)| *name_id == id));
        return (name_matches && accepts_unit(id, metric.standard_unit)).then_some(id);
    }
    let candidates: Vec<_> = named?
        .iter()
        .filter(|(id, standard)| {
            accepts_unit(id, standard) && code.is_none_or(|code| !incompatible_lab_code(code, id))
        })
        .collect();
    if candidates.len() == 1 {
        Some(candidates[0].0)
    } else {
        None
    }
}

#[derive(Clone, serde::Deserialize, Serialize, sqlx::FromRow)]
pub struct CustomMetric {
    pub metric_id: String,
    pub name: String,
    pub standard_unit: String,
}

pub(crate) fn custom_unit(raw: &str) -> &str {
    canonical_unit(raw).unwrap_or_else(|| raw.trim())
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum UnitMatch {
    Incompatible,
    NotReported,
    Convertible,
    Exact,
}

#[derive(Serialize)]
pub(crate) struct MetricSuggestion<'a> {
    pub(crate) metric_id: &'a str,
    pub(crate) name: &'a str,
    pub(crate) standard_unit: &'a str,
    pub(crate) unit_match: UnitMatch,
}

fn name_similarity(printed: &[char], candidate: &[char]) -> usize {
    if printed == candidate {
        return 100;
    }
    if printed.len().min(candidate.len()) < 3 {
        return 0;
    }
    let mut distances: Vec<usize> = (0..=candidate.len()).collect();
    for (i, letter) in printed.iter().enumerate() {
        let mut diagonal = distances[0];
        distances[0] = i + 1;
        for (j, other) in candidate.iter().enumerate() {
            let above = distances[j + 1];
            distances[j + 1] = (above + 1)
                .min(distances[j] + 1)
                .min(diagonal + usize::from(letter != other));
            diagonal = above;
        }
    }
    100 - 100 * distances[candidate.len()] / printed.len().max(candidate.len())
}

pub(crate) fn report_suggestion_code(code: &str) -> Option<&'static str> {
    // Quantitative urine ketones can be reviewed against the existing
    // qualitative entry, but the code alone cannot authorize conversion.
    match code {
        "50557-8" => Some("urine_ketones"),
        _ => loinc_metric(code),
    }
}

pub(crate) fn metric_suggestions<'a>(
    raw_name: &str,
    raw_unit: Option<&str>,
    ocr_candidate: Option<&str>,
    custom_metrics: &'a [CustomMetric],
) -> Vec<MetricSuggestion<'a>> {
    let (printed, code) = split_test_code(raw_name);
    let coded = code.and_then(report_suggestion_code);
    let printed: Vec<char> = crate::laboratory_names::key(printed).chars().collect();
    let mut name_scores = std::collections::BTreeMap::new();
    if !printed.is_empty() && printed != ['t', 'e', 's', 't'] {
        for (alias, candidates) in ALIAS_INDEX.iter() {
            let alias: Vec<char> = alias.chars().collect();
            let score = name_similarity(&printed, &alias);
            if score >= 50 {
                for &(id, _) in candidates {
                    let best = name_scores.entry(id).or_insert(0);
                    *best = (*best).max(score);
                }
            }
        }
    }
    if let Some(candidate) = ocr_candidate.and_then(|id| DEFINITIONS.get(id)) {
        name_scores
            .entry(candidate.metric_id)
            .and_modify(|score| *score = (*score).max(85))
            .or_insert(85);
    }
    if let Some(id) = coded {
        name_scores.insert(id, 100);
    }
    let reported = raw_unit.map(str::trim).filter(|unit| !unit.is_empty());
    let mut ranked = Vec::new();
    let mut add =
        |id: &'a str, name: &'a str, standard_unit: &'a str, name_score: i32, is_custom: bool| {
            let unit_match = match reported {
                None => UnitMatch::NotReported,
                Some(unit) if unit == standard_unit => UnitMatch::Exact,
                Some(unit) if custom_unit(unit) == standard_unit => UnitMatch::Convertible,
                Some(unit)
                    if !is_custom
                        && canonical_unit(unit)
                            .is_some_and(|unit| conversion(id, standard_unit, unit).is_some()) =>
                {
                    UnitMatch::Convertible
                }
                Some(_) => UnitMatch::Incompatible,
            };
            let score = name_score
                + match unit_match {
                    UnitMatch::Exact => 24,
                    UnitMatch::Convertible => 18,
                    UnitMatch::NotReported => 0,
                    UnitMatch::Incompatible => -18,
                };
            ranked.push((
                score,
                MetricSuggestion {
                    metric_id: id,
                    name,
                    standard_unit,
                    unit_match,
                },
            ));
        };
    for (id, score) in name_scores {
        if let Some(metric) = DEFINITIONS.get(id) {
            add(id, metric.name, metric.standard_unit, score as i32, false);
        }
    }
    for metric in custom_metrics {
        let name: Vec<char> = crate::laboratory_names::key(&metric.name).chars().collect();
        let score = name_similarity(&printed, &name).max(
            if Some(metric.metric_id.as_str()) == ocr_candidate {
                85
            } else {
                0
            },
        );
        if score >= 50 {
            add(
                &metric.metric_id,
                &metric.name,
                &metric.standard_unit,
                score as i32,
                true,
            );
        }
    }
    ranked.sort_unstable_by(|(a_score, a), (b_score, b)| {
        (Some(b.metric_id) == ocr_candidate)
            .cmp(&(Some(a.metric_id) == ocr_candidate))
            .then_with(|| b_score.cmp(a_score))
            .then_with(|| b.unit_match.cmp(&a.unit_match))
            .then_with(|| a.metric_id.cmp(b.metric_id))
    });
    ranked
        .into_iter()
        .take(5)
        .map(|(_, suggestion)| suggestion)
        .collect()
}

pub fn canonical_unit(raw: &str) -> Option<&'static str> {
    let compact: String = raw
        .chars()
        .filter(|character| !character.is_whitespace())
        .map(|character| {
            if character == 'μ' || character == 'µ' {
                'u'
            } else {
                character
            }
        })
        .collect();
    // Closed spelling aliases. Do not globally lowercase SI prefixes (M is not m).
    match compact.as_str() {
        "1" | "ratio" | "Ratio" => Some("1"),
        "%bywt" => Some("% by wt"),
        "Angstrom" | "angstrom" | "Å" => Some("Angstrom"),
        "nmol/L" | "nmol/l" => Some("nmol/L"),
        "mcg/dL" | "ug/dL" | "ug/dl" | "ug/dL(calc)" | "mcg/dL(calc)" => Some("µg/dL"),
        "mcg/L" => Some("µg/L"),
        "cells/L" | "cell/L" | "count/L" | "{cells}/L" | "{count}/L" => Some("cells/L"),
        "cells/mL" | "cells/ml" | "cell/mL" | "count/mL" | "{cells}/mL" => Some("cells/mL"),
        "cells/uL" | "cell/uL" | "#/uL" | "count/uL" | "{cells}/uL" => Some("cells/µL"),
        "mIU/mL" | "mIU/ml" => Some("mIU/mL"),
        "IU/mL" | "IU/ml" | "[IU]/mL" => Some("IU/mL"),
        "U/mL" | "U/ml" => Some("U/mL"),
        "mg/dL" | "mg/dl" | "MG/DL" => Some("mg/dL"),
        "mg/L" | "mg/l" | "MG/L" => Some("mg/L"),
        "g/L" | "g/l" | "G/L" => Some("g/L"),
        "g/dL" | "g/dl" | "G/DL" | "g/dL(calc)" | "g/dl(calc)" => Some("g/dL"),
        "mmol/L" | "mmol/l" | "MMOL/L" => Some("mmol/L"),
        "umol/L" | "umol/l" | "UMOL/L" => Some("µmol/L"),
        "ng/mL" | "ng/ml" | "NG/ML" | "NG/mL" | "NG/ml" => Some("ng/mL"),
        "ug/L" | "ug/l" | "UG/L" => Some("µg/L"),
        "ug/mL" | "ug/ml" | "UG/ML" => Some("µg/mL"),
        "ng/L" | "ng/l" | "NG/L" => Some("ng/L"),
        "ng/dL" | "ng/dl" | "NG/dL" | "NG/dl" | "NG/DL" => Some("ng/dL"),
        "pg/mL" | "pg/ml" | "PG/mL" | "PG/ML" => Some("pg/mL"),
        "pg" | "PG" => Some("pg"),
        "mIU/L" | "mIU/l" | "uIU/mL" | "uIU/ml" | "u[IU]/mL" | "m[IU]/L" => Some("mIU/L"),
        "mL/min/1.73m2" | "ml/min/1.73m2" | "mL/min/{1.73_m2}" => Some("mL/min/1.73m2"),
        "s" | "sec" | "seconds" | "second(s)" => Some("s"),
        "/100WBC's" | "/100WBC’s" | "/100WBC" => Some("/100 WBC's"),
        // CBC reports also use the documented 10E6 and 10E3 unit spellings.
        "10E6/uL" | "x10E6/uL" | "10e6/uL" | "x10e6/uL" | "10^12/L" | "10*12/L" | "10¹²/L"
        | "10^6/uL" | "10*6/uL" | "x10(6)/uL" | "M/uL" | "10⁶/uL" | "×10⁶/uL" | "x10⁶/uL"
        | "×10^6/uL" | "x10^6/uL" | "10(6)/uL" | "10^6/microliter" | "10⁶/microliter"
        | "×10⁶/microliter" | "×10^6/microliter" | "10^6/microlitre" | "10⁶/microlitre"
        | "Million/uL" | "million/uL" | "10^6{cells}/uL" | "10*6{cells}/uL" | "10^12{cells}/L"
        | "10*12{cells}/L" => Some("10^12/L"),
        "%" | "%(calc)" | "%NGSP" | "NGSP%" | "Percent" | "percent" => Some("%"),
        "mmol/mol" | "MMOL/MOL" => Some("mmol/mol"),
        "U/L" | "IU/L" => Some("U/L"),
        "10E3/uL" | "x10E3/uL" | "10e3/uL" | "x10e3/uL" | "10^9/L" | "10⁹/L" | "10^3/uL"
        | "10³/uL" | "10*9/L" | "10*3/uL" | "K/uL" | "x10(3)/uL" | "x10^3/uL" | "×10^3/uL"
        | "×10³/uL" | "x10^9/L" | "x10(9)/L" | "×10^9/L" | "×10⁹/L" | "10(3)/uL"
        | "10^3/microliter" | "10³/microliter" | "10^3/microlitre" | "10³/microlitre"
        | "Thousand/uL" | "thousand/uL" | "10^3{cells}/uL" | "10*3{cells}/uL" | "10^9{cells}/L"
        | "10*9{cells}/L" => Some("10^9/L"),
        _ => crate::apple_health::volume_unit(&compact).map(|(unit, _)| unit),
    }
}

struct Conversion {
    scale: Decimal,
    divisor: Decimal,
    offset: Decimal,
    rule: &'static str,
    source: &'static str,
}
fn conversion(metric: &str, standard: &str, unit: &str) -> Option<Conversion> {
    let mut rule = Conversion {
        scale: Decimal::ONE,
        divisor: Decimal::ONE,
        offset: Decimal::ZERO,
        rule: "identity",
        source: "https://www.nist.gov/pml/owm/metric-si-prefixes",
    };
    if standard == unit {
        return Some(rule);
    }
    if (metric, standard, unit) == ("vitamin_d_25_oh_total", "ng/mL", "nmol/L") {
        rule.scale = Decimal::new(4, 1);
        rule.rule = "vitamin-d-25-oh-molar";
        rule.source = "https://ods.od.nih.gov/factsheets/VitaminD-HealthProfessional/";
        return Some(rule);
    }
    if let Some(exponent) = match standard {
        "10^9/L" => Some(9),
        "10^12/L" => Some(12),
        _ => None,
    } {
        let input_exponent = match unit {
            "cells/L" => 0,
            "cells/mL" => 3,
            "cells/µL" => 6,
            "10^9/L" => 9,
            "10^12/L" => 12,
            _ => return None,
        };
        if input_exponent < exponent {
            rule.scale = Decimal::new(1, exponent - input_exponent);
        } else {
            rule.scale = Decimal::from(10_i64.pow(input_exponent - exponent));
        }
        rule.rule = "cell-count-volume";
        return Some(rule);
    }
    if let (Some((_, target)), Some((_, input))) = (
        crate::apple_health::volume_unit(standard),
        crate::apple_health::volume_unit(unit),
    ) {
        let exponent = input - target;
        if exponent < 0 {
            rule.scale = Decimal::new(1, exponent.unsigned_abs());
        } else {
            rule.scale = Decimal::from(10_i64.pow(exponent as u32));
        }
        rule.rule = "si-volume";
        return Some(rule);
    }
    if metric == "hba1c" {
        if unit != "mmol/mol" {
            return None;
        }
        rule.scale = Decimal::new(9148, 5);
        rule.offset = Decimal::new(2152, 3);
        rule.rule = "hba1c-ifcc-ngsp";
        rule.source = "https://ngsp.org/ifccngsp.asp";
        return Some(rule);
    }
    if standard == "mg/dL" {
        rule.scale = match unit {
            "mg/L" | "µg/mL" => Decimal::new(1, 1),
            "µg/L" => Decimal::new(1, 4),
            "g/L" => Decimal::new(100, 0),
            "g/dL" => Decimal::new(1000, 0),
            "mmol/L" | "µmol/L" => {
                rule.divisor = match metric {
                    "total_cholesterol" | "ldl_cholesterol" | "hdl_cholesterol" => {
                        rule.source = "https://wwwn.cdc.gov/nchs/data/nhanes/public/2017/datafiles/p_trigly.htm";
                        rule.rule = "cholesterol-molar";
                        Decimal::new(2586, 5)
                    }
                    "triglycerides" => {
                        rule.source = "https://wwwn.cdc.gov/nchs/data/nhanes/public/2017/datafiles/p_trigly.htm";
                        rule.rule = "triglycerides-molar";
                        Decimal::new(1129, 5)
                    }
                    "glucose" => {
                        rule.source =
                            "https://wwwn.cdc.gov/Nchs/Data/Nhanes/Public/2009/DataFiles/GLU_F.htm";
                        rule.rule = "glucose-molar";
                        Decimal::new(5551, 5)
                    }
                    "creatinine" => {
                        rule.source = "https://iris.who.int/bitstream/handle/10665/333647/TLS-NTP-manual-eng.pdf";
                        rule.rule = "creatinine-molar";
                        Decimal::new(884, 4)
                    }
                    _ => return None,
                };
                if unit == "µmol/L" {
                    rule.divisor = rule.divisor.checked_mul(Decimal::new(1000, 0))?;
                }
                Decimal::ONE
            }
            _ => return None,
        };
    } else if standard == "g/dL" {
        rule.scale = match unit {
            "g/L" => Decimal::new(1, 1),
            "mg/dL" => Decimal::new(1, 3),
            "mg/L" => Decimal::new(1, 4),
            _ => return None,
        };
    } else if standard == "ng/mL" {
        rule.scale = match unit {
            "µg/L" => Decimal::ONE,
            "µg/mL" | "mg/L" => Decimal::new(1000, 0),
            "ng/L" => Decimal::new(1, 3),
            _ => return None,
        };
    } else {
        return None;
    }
    if rule.rule == "identity" {
        rule.rule = "mass-volume-si";
    }
    Some(rule)
}
impl Conversion {
    fn apply(&self, value: Decimal) -> Option<String> {
        let value = value
            .checked_mul(self.scale)?
            .checked_div(self.divisor)?
            .checked_add(self.offset)?;
        let rounded = value.round_dp(6);
        let retained = if (self.scale == Decimal::ONE
            && self.divisor == Decimal::ONE
            && self.offset == Decimal::ZERO)
            || (value != Decimal::ZERO && rounded == Decimal::ZERO)
        {
            value
        } else {
            rounded
        };
        Some(retained.normalize().to_string())
    }
}

#[derive(Serialize)]
pub struct Quantity {
    pub status: &'static str,
    pub reason: &'static str,
    pub value: Option<String>,
    pub lower: Option<String>,
    pub upper: Option<String>,
    pub lower_inclusive: Option<bool>,
    pub upper_inclusive: Option<bool>,
    pub unit: Option<String>,
    pub original_unit: Option<String>,
    pub unit_origin: &'static str,
    pub display: Option<String>,
    pub rule: Option<&'static str>,
    pub source: Option<&'static str>,
}
impl Quantity {
    fn unavailable(
        status: &'static str,
        original_unit: Option<&str>,
        origin: &'static str,
    ) -> Self {
        let reason = match status {
            "unmapped_metric" => {
                "Map the test to a known metric after checking the original report."
            }
            "missing_unit" => "No printed unit is available; do not infer one.",
            "unsupported_unit" => "This unit is not supported for the selected metric.",
            "unparsed_value" => {
                "Text, ambiguous numbers or complex ranges are retained without an exact numeric point."
            }
            "unparsed_reference" => {
                "The reference includes unsupported syntax or conditions; retain the original wording."
            }
            "missing_reference" => "No reference interval was printed.",
            "overflow" => "The numeric value exceeds the supported decimal range.",
            _ => "The original value cannot be compared.",
        };
        Self {
            status,
            reason,
            value: None,
            lower: None,
            upper: None,
            lower_inclusive: None,
            upper_inclusive: None,
            unit: None,
            original_unit: original_unit.map(str::to_owned),
            unit_origin: origin,
            display: None,
            rule: None,
            source: None,
        }
    }
}
#[derive(Serialize)]
pub struct Interpretation {
    pub version: &'static str,
    pub result: Quantity,
    pub reference: Quantity,
}

enum Shape {
    Exact(Decimal),
    Below(Decimal, bool),
    Above(Decimal, bool),
    Range(Decimal, Decimal),
}
fn decimal(text: &str) -> Option<Decimal> {
    let text = text.trim();
    if text.is_empty()
        || text.len() > 64
        || !text
            .chars()
            .all(|c| c.is_ascii_digit() || ".+-eE".contains(c))
        || !text.chars().any(|c| c.is_ascii_digit())
    {
        return None;
    }
    let value = if text.contains(['e', 'E']) {
        Decimal::from_scientific(text).ok()?
    } else {
        Decimal::from_str(text).ok()?
    };
    (value >= Decimal::ZERO).then_some(value)
}
fn shape(text: &str) -> Option<Shape> {
    let text = text.trim();
    for (prefix, above, inclusive) in [
        ("<=", false, true),
        (">=", true, true),
        ("≤", false, true),
        ("≥", true, true),
        ("<", false, false),
        (">", true, false),
    ] {
        if let Some(rest) = text.strip_prefix(prefix) {
            return decimal(rest).map(|value| {
                if above {
                    Shape::Above(value, inclusive)
                } else {
                    Shape::Below(value, inclusive)
                }
            });
        }
    }
    if let Some(value) = decimal(text) {
        return Some(Shape::Exact(value));
    }
    for separator in ["–", "—", " to ", "-"] {
        if let Some((left, right)) = text.split_once(separator)
            && let (Some(left), Some(right)) = (decimal(left), decimal(right))
            && left <= right
        {
            return Some(Shape::Range(left, right));
        }
    }
    None
}
// Accept a printed unit suffix only when it agrees with the independently
// supplied unit. Never take a numeric prefix from arbitrary text or mismatched units.
fn result_shape(raw: &str, unit: &str) -> Option<Shape> {
    let raw = raw.trim();
    let raw = raw.strip_suffix("(calc)").unwrap_or(raw).trim_end();
    if let Some(parsed) = shape(raw) {
        return Some(parsed);
    }
    raw.char_indices().find_map(|(index, _)| {
        let (value, suffix) = raw.split_at(index);
        if custom_unit(suffix) == unit {
            shape(value)
        } else {
            None
        }
    })
}

fn derive(
    metric_id: Option<&str>,
    custom: Option<&CustomMetric>,
    raw: &str,
    raw_unit: Option<&str>,
    origin: &'static str,
    reference: bool,
) -> Quantity {
    let Some((id, standard_unit)) = custom
        .map(|metric| (metric.metric_id.as_str(), metric.standard_unit.as_str()))
        .or_else(|| {
            metric_id
                .and_then(|id| DEFINITIONS.get(id))
                .map(|metric| (metric.metric_id, metric.standard_unit))
        })
    else {
        return Quantity::unavailable("unmapped_metric", raw_unit, origin);
    };
    let Some(raw_unit) = raw_unit
        .filter(|unit| !unit.trim().is_empty())
        .or((custom.is_none() && standard_unit == "1").then_some("1"))
    else {
        return Quantity::unavailable("missing_unit", raw_unit, origin);
    };
    let unit = custom_unit(raw_unit);
    let rule = if custom.is_some() {
        (unit == standard_unit && !standard_unit.is_empty())
            .then(|| conversion(id, standard_unit, unit))
            .flatten()
    } else {
        canonical_unit(raw_unit).and_then(|unit| conversion(id, standard_unit, unit))
    };
    let Some(rule) = rule else {
        return Quantity::unavailable("unsupported_unit", Some(raw_unit), origin);
    };
    let Some(parsed) = (if reference {
        shape(raw)
    } else {
        result_shape(raw, unit)
    }) else {
        return Quantity::unavailable(
            if reference {
                "unparsed_reference"
            } else {
                "unparsed_value"
            },
            Some(raw_unit),
            origin,
        );
    };
    let mut quantity = Quantity::unavailable("overflow", Some(raw_unit), origin);
    quantity.unit = Some(standard_unit.to_owned());
    quantity.rule = Some(rule.rule);
    quantity.source = Some(rule.source);
    let display = match parsed {
        Shape::Exact(value) => {
            let Some(value) = rule.apply(value) else {
                return quantity;
            };
            quantity.status = "exact";
            quantity.value = Some(value.clone());
            value
        }
        Shape::Below(value, inclusive) => {
            let Some(value) = rule.apply(value) else {
                return quantity;
            };
            quantity.status = "comparison";
            quantity.upper = Some(value.clone());
            quantity.upper_inclusive = Some(inclusive);
            format!("{} {value}", if inclusive { "≤" } else { "<" })
        }
        Shape::Above(value, inclusive) => {
            let Some(value) = rule.apply(value) else {
                return quantity;
            };
            quantity.status = "comparison";
            quantity.lower = Some(value.clone());
            quantity.lower_inclusive = Some(inclusive);
            format!("{} {value}", if inclusive { "≥" } else { ">" })
        }
        Shape::Range(left, right) => {
            let (Some(left), Some(right)) = (rule.apply(left), rule.apply(right)) else {
                return quantity;
            };
            quantity.status = "range";
            quantity.lower = Some(left.clone());
            quantity.upper = Some(right.clone());
            quantity.lower_inclusive = Some(true);
            quantity.upper_inclusive = Some(true);
            format!("{left} – {right}")
        }
    };
    quantity.display = Some(format!("{display} {standard_unit}"));
    quantity.reason = if quantity.status == "exact" {
        "Deterministic conversion; original value retained."
    } else {
        "Bounded value retained; it is not an exact numeric trend point."
    };
    quantity
}

pub fn interpret(payload: &ObservationPayload) -> Interpretation {
    interpret_with_custom(payload, None)
}

pub(crate) fn interpret_with_custom(
    payload: &ObservationPayload,
    custom: Option<&CustomMetric>,
) -> Interpretation {
    interpret_metric(payload, resolved_metric(payload), custom)
}

pub(crate) fn interpret_metric(
    payload: &ObservationPayload,
    metric: Option<&str>,
    custom: Option<&CustomMetric>,
) -> Interpretation {
    let result = derive(
        metric,
        custom,
        &payload.raw_result,
        payload.raw_unit.as_deref(),
        "observation",
        false,
    );
    let reference = match payload
        .reference_range
        .as_deref()
        .filter(|text| !text.trim().is_empty())
    {
        None => Quantity::unavailable("missing_reference", None, "none"),
        Some(raw) => {
            let explicit = raw.char_indices().find_map(|(index, _)| {
                let (value, unit) = raw.split_at(index);
                (canonical_unit(unit).is_some() && shape(value).is_some()).then_some((value, unit))
            });
            if let Some((value, unit)) = explicit {
                derive(
                    metric,
                    custom,
                    value,
                    Some(unit),
                    "reference_explicit",
                    true,
                )
            } else {
                derive(
                    metric,
                    custom,
                    raw,
                    payload.raw_unit.as_deref(),
                    "observation",
                    true,
                )
            }
        }
    };
    Interpretation {
        version: CONVERSION_VERSION,
        result,
        reference,
    }
}

pub fn normalized(payload: &ObservationPayload) -> Option<(String, String)> {
    let interpreted = interpret(payload).result;
    if interpreted.status != "exact" {
        return None;
    }
    Some((interpreted.value?, interpreted.unit?))
}
