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

#[derive(sqlx::FromRow)]
pub(crate) struct NamedReportMetric {
    metric_id: String,
    pub printed_name: String,
    standard_unit: String,
    pub loinc_code: Option<String>,
}

pub(crate) fn is_reported_id(id: &str) -> bool {
    id.strip_prefix(crate::reports::REPORTED_METRIC_PREFIX)
        .is_some_and(|key| key.len() == 64 && key.bytes().all(|b| b.is_ascii_hexdigit()))
}

#[derive(Default)]
pub(crate) struct Metadata {
    pub aliases: BTreeMap<String, String>,
    pub links: BTreeMap<String, String>,
    pub named_reports: BTreeMap<String, NamedReportMetric>,
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
            named_reports: sqlx::query_as::<_, NamedReportMetric>(queries::NAMED_REPORT_METRICS)
                .bind(user_id)
                .fetch_all(&mut *connection)
                .await?
                .into_iter()
                .map(|metric| (metric.metric_id.clone(), metric))
                .collect(),
        })
    }

    pub fn named_report_metrics(&self) -> impl Iterator<Item = CustomMetric> + '_ {
        self.named_reports.iter().map(|(id, metric)| CustomMetric {
            metric_id: id.clone(),
            name: self.aliases[id].clone(),
            standard_unit: metric.standard_unit.clone(),
        })
    }

    pub fn report_suggestion<'a>(
        &'a self,
        payload: &ObservationPayload,
        source: Option<&str>,
        report_id: &str,
    ) -> Option<&'a str> {
        let code = test_code(payload);
        if code.is_some_and(|code| loinc_metric(code).is_some() || self.links.contains_key(code)) {
            return None;
        }
        let identity = crate::reports::reported_identity(payload, source, report_id);
        if let Some((key, _)) = self.named_reports.get_key_value(&identity) {
            return Some(key);
        }
        let code = code?;
        let mut candidates = self
            .named_reports
            .iter()
            .filter(|(_, metric)| metric.loinc_code.as_deref() == Some(code));
        let (id, _) = candidates.next()?;
        candidates.next().is_none().then_some(id)
    }

    pub fn apply_name(&self, metric: &mut Value) {
        if let Some(id) = metric["metric_id"].as_str()
            && let Some(name) = self.aliases.get(id)
        {
            metric["original_name"] = self
                .named_reports
                .get(id)
                .map_or_else(|| metric["name"].clone(), |named| json!(named.printed_name));
            metric["name"] = json!(name);
        }
    }

    pub fn apply_loinc(&self, metric: &mut Value, native_codes: Option<&BTreeSet<String>>) {
        let Some(id) = metric["metric_id"].as_str() else {
            return;
        };
        metric["loinc"] =
            self.loinc_entries(id, native_codes.into_iter().flatten().map(String::as_str));
    }

    pub(crate) fn loinc_entries<'a>(
        &'a self,
        id: &str,
        native_codes: impl Iterator<Item = &'a str>,
    ) -> Value {
        json!(
            self.codes_for_metric(id, native_codes)
                .into_iter()
                .map(|code| json!({
                    "code": code,
                    "official_name": crate::loinc::long_common_name(code),
                }))
                .collect::<Vec<_>>()
        )
    }

    pub(crate) fn codes_for_metric<'a>(
        &'a self,
        id: &str,
        native_codes: impl Iterator<Item = &'a str>,
    ) -> BTreeSet<&'a str> {
        LOINC_METRICS
            .iter()
            .filter_map(|&(code, metric)| (metric == id).then_some(code))
            .chain(self.links.iter().filter_map(|(code, metric)| {
                (metric == id && loinc_metric(code).is_none()).then_some(code.as_str())
            }))
            .chain(native_codes)
            .chain(
                self.named_reports
                    .get(id)
                    .and_then(|metric| metric.loinc_code.as_deref()),
            )
            .collect()
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

pub(crate) fn native_codes_for_metric<'a>(
    links: &'a BTreeMap<String, BTreeSet<String>>,
    id: &'a str,
) -> impl Iterator<Item = &'a str> {
    links
        .iter()
        .filter_map(|(code, ids)| ids.contains(id).then_some(code.as_str()))
}

pub(crate) struct LinkTarget<'a> {
    pub id: &'a str,
    pub standard_unit: Option<&'a str>,
    pub is_new: bool,
}

pub(crate) async fn link_submitted(
    transaction: &mut Transaction<'_, Sqlite>,
    user_id: &str,
    metadata: &mut Metadata,
    native_links: &BTreeMap<String, BTreeSet<String>>,
    target: LinkTarget<'_>,
    code: &str,
    observation: &ObservationPayload,
) -> Result<bool, AppError> {
    let LinkTarget {
        id,
        standard_unit,
        is_new,
    } = target;
    let existing = metadata.codes_for_metric(id, native_codes_for_metric(native_links, id));
    let bound = loinc_metric(code).or_else(|| metadata.links.get(code).map(String::as_str));
    if bound == Some(id) {
        return Ok(false);
    }
    let native_conflict = native_links
        .get(code)
        .is_some_and(|ids| ids.len() != 1 || !ids.contains(id));
    if bound.is_some() || native_conflict {
        return Err(AppError::LoincCodeConflict {
            incoming: code.to_owned(),
            existing: existing.iter().copied().collect::<Vec<_>>().join(", "),
            native: bound.is_none(),
        });
    }
    crate::loinc::validate_association(code, &existing, id, standard_unit, observation)?;
    if existing.is_empty() && crate::loinc::long_common_name(code).is_none() && !is_new {
        return Ok(false);
    }
    sqlx::query(queries::LINK_LAB_METRIC_LOINC)
        .bind(user_id)
        .bind(code)
        .bind(id)
        .execute(&mut **transaction)
        .await?;
    metadata.links.insert(code.to_owned(), id.to_owned());
    Ok(true)
}

async fn register_named_report_metric(
    connection: &mut SqliteConnection,
    user: &str,
    definition: &Value,
) -> Result<(), AppError> {
    let id = definition["metric_id"].as_str().ok_or(AppError::Internal)?;
    let name = definition["original_name"]
        .as_str()
        .or_else(|| definition["name"].as_str())
        .ok_or(AppError::Internal)?;
    let unit = definition["standard_unit"]
        .as_str()
        .map(crate::laboratory::custom_unit)
        .filter(|unit| !unit.is_empty())
        .unwrap_or("1");
    let code = definition["loinc"][0]["code"]
        .as_str()
        .filter(|code| crate::loinc::long_common_name(code).is_some());
    sqlx::query(queries::INSERT_NAMED_REPORT_METRIC)
        .bind(user)
        .bind(id)
        .bind(name)
        .bind(unit)
        .bind(code)
        .execute(connection)
        .await?;
    Ok(())
}

pub(crate) async fn register_existing_report_aliases(database: &Database) -> Result<(), AppError> {
    let owners: Vec<String> = sqlx::query_scalar(queries::UNREGISTERED_REPORT_ALIAS_OWNERS)
        .fetch_all(&database.pool)
        .await?;
    for user in owners {
        let mut transaction = database.pool.begin_with("BEGIN IMMEDIATE").await?;
        let metadata = Metadata::load(&mut transaction, &user).await?;
        let mut remaining: BTreeSet<_> = metadata
            .aliases
            .keys()
            .filter(|id| is_reported_id(id) && !metadata.named_reports.contains_key(*id))
            .collect();
        let originals: Vec<(String, String, String)> =
            sqlx::query_as(queries::NAMED_REPORT_METRIC_ORIGINS)
                .bind(&user)
                .fetch_all(&mut *transaction)
                .await?;
        for (report_id, payload, context) in originals {
            let payload: ObservationPayload = serde_json::from_str(&payload)?;
            let context: Value = serde_json::from_str(&context)?;
            let id = crate::reports::reported_identity(
                &payload,
                context["source_name"].as_str(),
                &report_id,
            );
            if !remaining.remove(&id) {
                continue;
            }
            register_named_report_metric(
                &mut transaction,
                &user,
                &json!({
                    "metric_id": id,
                    "name": split_test_code(&payload.raw_name).0,
                    "standard_unit": payload.raw_unit,
                    "loinc": [{"code": test_code(&payload)}],
                }),
            )
            .await?;
            if remaining.is_empty() {
                break;
            }
        }
        transaction.commit().await?;
    }
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
    let definition = catalog["metrics"]
        .as_array()
        .ok_or(AppError::Internal)?
        .iter()
        .find(|entry| entry["metric_id"] == request.metric_id)
        .ok_or(AppError::NotFound)?;
    let mut transaction = database.pool.begin_with("BEGIN IMMEDIATE").await?;
    sqlx::query(queries::SET_LAB_METRIC_ALIAS)
        .bind(user)
        .bind(&request.metric_id)
        .bind(name)
        .execute(&mut *transaction)
        .await?;
    if is_reported_id(&request.metric_id) {
        register_named_report_metric(&mut transaction, user, definition).await?;
    }
    sqlx::query(queries::INVALIDATE_EXPORTS)
        .bind(user)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(json!({"metric_id": request.metric_id, "name": name}))
}
