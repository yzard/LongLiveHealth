use crate::{
    clinical_labs::Group,
    database::{Database, queries},
    error::AppError,
    laboratory::{CustomMetric, LOINC_METRICS, loinc_metric, split_test_code},
    metric_history,
    reports::ObservationPayload,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{Sqlite, SqliteConnection, Transaction};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(crate) struct Metadata {
    pub aliases: BTreeMap<String, String>,
    pub links: BTreeMap<String, String>,
}

impl Metadata {
    pub async fn load(connection: &mut SqliteConnection, user_id: &str) -> Result<Self, AppError> {
        Ok(Self {
            aliases: sqlx::query_as(queries::LAB_METRIC_ALIASES)
                .bind(user_id)
                .fetch_all(&mut *connection)
                .await?
                .into_iter()
                .collect(),
            links: sqlx::query_as(queries::LAB_METRIC_LOINC_LINKS)
                .bind(user_id)
                .fetch_all(&mut *connection)
                .await?
                .into_iter()
                .collect(),
        })
    }

    pub fn apply_name(&self, metric: &mut Value) {
        if let Some(id) = metric["metric_id"].as_str()
            && let Some(name) = self.aliases.get(id)
        {
            metric["original_name"] = metric["name"].clone();
            metric["name"] = json!(name);
        }
    }

    pub fn apply_loinc(&self, metric: &mut Value, native_codes: Option<&BTreeSet<String>>) {
        let Some(id) = metric["metric_id"].as_str() else {
            return;
        };
        let codes: BTreeSet<_> = LOINC_METRICS
            .iter()
            .filter_map(|&(code, metric)| (metric == id).then_some(code))
            .chain(self.links.iter().filter_map(|(code, metric)| {
                (metric == id && loinc_metric(code).is_none()).then_some(code.as_str())
            }))
            .chain(native_codes.into_iter().flatten().map(String::as_str))
            .collect();
        metric["loinc"] = json!(
            codes
                .into_iter()
                .map(|code| json!({
                    "code": code,
                    "official_name": crate::loinc::long_common_name(code),
                }))
                .collect::<Vec<_>>()
        );
    }

    pub fn linked_codes_by_metric(&self) -> BTreeMap<String, BTreeSet<String>> {
        let mut result = BTreeMap::<_, BTreeSet<_>>::new();
        for (code, id) in &self.links {
            result.entry(id.clone()).or_default().insert(code.clone());
        }
        result
    }
}

pub(crate) fn test_code(payload: &ObservationPayload) -> Option<&str> {
    split_test_code(&payload.raw_name).1.filter(|code| {
        loinc_metric(code).is_some() || crate::loinc::long_common_name(code).is_some()
    })
}

pub(crate) fn is_clinical_id(id: &str) -> bool {
    id.strip_prefix("clinical:")
        .is_some_and(|key| key.len() == 64 && key.bytes().all(|b| b.is_ascii_hexdigit()))
}

pub(crate) fn clinical_metric(group: &Group) -> Result<CustomMetric, AppError> {
    let mut metric: CustomMetric = serde_json::from_value(group.definition.clone())?;
    if metric.standard_unit.is_empty()
        && let Some(units) = group.definition["display_units"].as_array()
        && let [unit] = units.as_slice()
        && let Some(unit) = unit.as_str().filter(|unit| *unit != "Not reported")
    {
        metric.standard_unit = crate::laboratory::custom_unit(unit).to_owned();
    }
    Ok(metric)
}

pub(crate) async fn clinical_groups(
    database: &Database,
    user: &str,
) -> Result<BTreeMap<String, Group>, AppError> {
    let records = metric_history::load_health(&database.pool, user, "").await?;
    database
        .cpu
        .run(move || Ok(crate::clinical_labs::groups(&records)))
        .await
}

pub(crate) fn clinical_links(
    groups: &BTreeMap<String, Group>,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut links: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (id, group) in groups {
        for code in &group.loinc_codes {
            links.entry(code.clone()).or_default().insert(id.clone());
        }
    }
    links
}

pub(crate) async fn link_confirmed(
    transaction: &mut Transaction<'_, Sqlite>,
    user_id: &str,
    metadata: &mut Metadata,
    native_links: &BTreeMap<String, BTreeSet<String>>,
    payload: &ObservationPayload,
) -> Result<(), AppError> {
    let (Some(id), Some(code)) = (payload.metric_id.as_deref(), test_code(payload)) else {
        return Ok(());
    };
    if let Some(existing) =
        loinc_metric(code).or_else(|| metadata.links.get(code).map(String::as_str))
    {
        if existing == id {
            return Ok(());
        }
        return Err(AppError::Invalid(
            "This LOINC is already linked to another Browse metric. Check the printed code and metric mapping.",
        ));
    }
    if native_links
        .get(code)
        .is_some_and(|ids| ids.len() != 1 || !ids.contains(id))
    {
        return Err(AppError::Invalid(
            "This LOINC is already used by a different or ambiguous Apple Health metric. Check the printed code and metric mapping.",
        ));
    }
    sqlx::query(queries::LINK_LAB_METRIC_LOINC)
        .bind(user_id)
        .bind(code)
        .bind(id)
        .execute(&mut **transaction)
        .await?;
    metadata.links.insert(code.to_owned(), id.to_owned());
    Ok(())
}

impl Database {
    pub(crate) async fn lab_metric_metadata(&self, user: &str) -> Result<Metadata, AppError> {
        Metadata::load(&mut *self.pool.acquire().await?, user).await
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RenameRequest {
    metric_id: String,
    name: String,
}

pub(crate) async fn rename(
    database: &Database,
    user: &str,
    request: RenameRequest,
) -> Result<Value, AppError> {
    let name = request.name.trim();
    if name.is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
        return Err(AppError::Invalid(
            "Enter a metric name of 1–256 bytes without control characters",
        ));
    }
    let catalog = metric_history::browse(database, user).await?;
    if !catalog["metrics"]
        .as_array()
        .ok_or(AppError::Internal)?
        .iter()
        .any(|entry| entry["metric_id"] == request.metric_id)
    {
        return Err(AppError::NotFound);
    }
    let mut transaction = database.pool.begin_with("BEGIN IMMEDIATE").await?;
    sqlx::query(queries::SET_LAB_METRIC_ALIAS)
        .bind(user)
        .bind(&request.metric_id)
        .bind(name)
        .execute(&mut *transaction)
        .await?;
    sqlx::query(queries::INVALIDATE_EXPORTS)
        .bind(user)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(json!({"metric_id": request.metric_id, "name": name}))
}
