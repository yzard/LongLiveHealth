use crate::ocr::ExtractedPage;
use crate::{
    app::AppState,
    authentication::{digest, now},
    database::queries,
    error::AppError,
    jobs::ClaimedJob,
    provider,
};
use base64::Engine;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

type ObservationPayload = crate::reports::ObservationPayload;

#[derive(sqlx::FromRow)]
struct StoredCandidate {
    observation_id: String,
    candidate_key: Option<String>,
    editable: bool,
    payload_json: String,
}

fn same_printed_result(a: &ObservationPayload, b: &ObservationPayload) -> bool {
    same_printed_identity(a, b) && a.raw_result.trim() == b.raw_result.trim()
}

fn same_printed_identity(a: &ObservationPayload, b: &ObservationPayload) -> bool {
    a.source.page == b.source.page
        && crate::laboratory_names::key(&a.raw_name) == crate::laboratory_names::key(&b.raw_name)
        && a.raw_unit.as_deref().map(str::trim) == b.raw_unit.as_deref().map(str::trim)
        && a.sampled_at == b.sampled_at
}

pub async fn run(state: AppState, stop: CancellationToken) {
    let tasks = async {
        tokio::join!(
            extraction_loop(state.clone()),
            maintenance_loop(state.clone()),
            analysis_loop(state.clone()),
            report_analysis_loop(state.clone()),
            export_loop(state.clone()),
            daily_view_loop(state)
        );
    };
    tokio::select! {_ = stop.cancelled()=>{}, _ = tasks=>{}}
}

async fn maintenance_loop(state: AppState) {
    let mut tick = tokio::time::interval(Duration::from_secs(2));
    loop {
        tick.tick().await;
        if let Err(error) = crate::maintenance::cleanup(&state).await {
            tracing::warn!(code=%error,"cleanup will retry");
        }
    }
}
async fn analysis_loop(state: AppState) {
    let mut tick = tokio::time::interval(Duration::from_secs(2));
    loop {
        tick.tick().await;
        if let Err(error) = crate::analysis::analyze_next(&state).await {
            tracing::warn!(code=%error,"analysis worker failed");
        }
    }
}
async fn report_analysis_loop(state: AppState) {
    let mut tick = tokio::time::interval(Duration::from_secs(2));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        if let Err(error) = crate::analysis::prepare_next_report_analysis(&state).await {
            tracing::warn!(code=%error,"report analysis snapshot worker will retry");
        }
    }
}

async fn export_loop(state: AppState) {
    let mut tick = tokio::time::interval(Duration::from_secs(2));
    loop {
        tick.tick().await;
        if let Err(error) = crate::maintenance::export_next(&state).await {
            tracing::warn!(code=%error,"export worker failed");
        }
    }
}

async fn extraction_loop(state: AppState) {
    let mut tick = tokio::time::interval(Duration::from_secs(2));
    loop {
        tick.tick().await;
        if let Err(error) = extract_next(&state).await {
            tracing::warn!(code=%error,"document task could not finish");
        }
    }
}

pub async fn extract_next(state: &AppState) -> Result<bool, AppError> {
    let Some(job) = state
        .database
        .claim_job(
            now()?,
            state.config.jobs.lease_seconds,
            state.config.jobs.maximum_attempts,
        )
        .await?
    else {
        return Ok(false);
    };
    let renewal = async {
        let period = Duration::from_secs((state.config.jobs.lease_seconds / 3).max(1) as u64);
        let mut heartbeat = tokio::time::interval_at(tokio::time::Instant::now() + period, period);
        loop {
            heartbeat.tick().await;
            if let Err(error) = state
                .database
                .renew_job(&job, now()?, state.config.jobs.lease_seconds)
                .await
            {
                break Err::<(), AppError>(error);
            }
        }
    };
    // Poll both futures throughout renewal: extraction may own a SQLite write transaction.
    // Suspending it while awaiting renewal would prevent that transaction from committing.
    let result = tokio::select! {
        result = extract(state, &job) => result,
        renewal_result = renewal => { renewal_result?; return Err(AppError::Internal); }
    };
    if let Err(error) = &result {
        tracing::warn!(job_id=%job.job_id,code=%error,"document extraction failed");
    }
    let code = result.as_ref().err().map(|_| "extraction_failed");
    state.database.finish_job(&job, now()?, code).await?;
    Ok(true)
}

async fn extract(state: &AppState, job: &ClaimedJob) -> Result<(), AppError> {
    state
        .database
        .report_summary(&job.user_id, &job.file_id)
        .await?;
    let file = state.database.file(&job.user_id, &job.file_id).await?;
    let scratch = state.database.temporary_files.reserve(
        state
            .config
            .data_dir
            .join("tmp")
            .join(&job.user_id)
            .join(format!("{}-{}", job.job_id, uuid::Uuid::new_v4())),
    )?;
    let source = scratch.path().join("processing-original");
    {
        let _guard = state.file_mutation.lock().await;
        let current = state.database.file(&job.user_id, &job.file_id).await?;
        tokio::fs::create_dir_all(scratch.path()).await?;
        // Pin the inode for the whole task. Naming or user edits may relocate the
        // archive and cleanup may unlink its old path while subsequent pages render.
        tokio::fs::hard_link(
            state.config.data_dir.join(
                current
                    .processing_path
                    .as_ref()
                    .unwrap_or(&current.relative_path),
            ),
            &source,
        )
        .await?;
    }
    let database_metrics = crate::ocr::database_metrics(&state.database, &job.user_id).await?;
    let run_id = uuid::Uuid::new_v4().to_string();
    let mut failed = false;
    let pdf = file.content_type == "application/pdf";
    let mut first_evidence = if pdf {
        Some(crate::documents::pdf_evidence(&state.database.cpu, &source, 1).await?)
    } else {
        None
    };
    let report_collection_date = match &first_evidence {
        Some(evidence) => {
            crate::documents::pdf_collection_date(
                &state.database.cpu,
                &source,
                file.page_count,
                evidence,
            )
            .await?
        }
        None => None,
    };
    for page in 1..=file.page_count {
        let mut evidence = match first_evidence.take() {
            Some(evidence) => evidence,
            None if pdf => {
                crate::documents::pdf_evidence(&state.database.cpu, &source, page).await?
            }
            None => crate::documents::PageEvidence::image(),
        };
        evidence.report_collection_date = report_collection_date.clone();
        save_input(state, job, &run_id, page, &evidence).await?;
        let image = if file.content_type == "application/pdf" {
            let prefix = scratch.path().join(format!("page-{page}"));
            let mut command = tokio::process::Command::new("pdftoppm");
            command
                .args([
                    "-f",
                    &page.to_string(),
                    "-l",
                    &page.to_string(),
                    "-singlefile",
                    "-scale-to",
                    "2400",
                    "-png",
                ])
                .arg(&source)
                .arg(&prefix)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .kill_on_drop(true);
            let status = tokio::time::timeout(Duration::from_secs(60), command.status())
                .await
                .map_err(|_| AppError::Invalid("PDF rendering timed out"))??;
            if !status.success() {
                return Err(AppError::Invalid("PDF rendering failed"));
            }
            prefix.with_extension("png")
        } else {
            source.clone()
        };
        let bytes = tokio::fs::read(&image).await?;
        if bytes.len() > 40 * 1024 * 1024 {
            return Err(AppError::TooLarge);
        }
        let mime = if file.processing_path.is_some() || file.content_type == "image/jpeg" {
            "image/jpeg"
        } else {
            "image/png"
        };
        let image_url = state
            .database
            .cpu
            .run_background(move || {
                Ok(format!(
                    "data:{mime};base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(bytes)
                ))
            })
            .await?;
        let page_output = extract_page(
            state,
            job,
            &run_id,
            page,
            image_url,
            &evidence,
            database_metrics.clone(),
        )
        .await;
        let mut identity = (None, None);
        let (candidates, content, model, status) = match page_output {
            Ok((page_output, content, model)) => {
                identity = (page_output.report_title, page_output.source_name);
                let mut valid = Vec::new();
                let incomplete = !crate::ocr::validation_issues(
                    &content,
                    page,
                    report_collection_date.is_some(),
                )
                .is_empty();
                let mut page_failed = evidence.error_code.is_some() || incomplete;
                failed |= incomplete;
                if page_output.observations.len() > 2000 {
                    return Err(AppError::TooLarge);
                }
                for mut observation in page_output.observations {
                    if crate::laboratory::is_administrative_name(&observation.raw_name) {
                        continue;
                    }
                    observation.source.page = page;
                    page_failed |= observation
                        .ocr_mapping_note
                        .as_ref()
                        .is_some_and(|note| !note.trim().is_empty());
                    if observation
                        .sampled_at
                        .as_ref()
                        .is_none_or(|date| date.trim().is_empty())
                        && let Some(collection) = &report_collection_date
                    {
                        observation.sampled_at = Some(collection.date.clone());
                    }
                    // Model-supplied IDs are not trusted. The shared deterministic catalog
                    // derives resolved_metric_id while retaining the printed payload.
                    observation.metric_id = None;
                    if observation.validate(file.page_count).is_err() {
                        page_failed = true;
                        failed = true;
                        continue;
                    }
                    valid.push(observation);
                }
                if !page_output.warnings.is_empty() {
                    page_failed = true;
                }
                // Review warnings are evidence, not an execution failure. Candidates remain pending.
                (
                    valid,
                    content,
                    model,
                    if page_failed {
                        "needs_review"
                    } else {
                        "extracted"
                    },
                )
            }
            Err(_) => {
                failed = true;
                (
                    Vec::new(),
                    "Page extraction failed; retry or enter results manually".into(),
                    "unknown".into(),
                    "failed",
                )
            }
        };
        let mut transaction = state.database.pool.begin_with("BEGIN IMMEDIATE").await?;
        let active: i64 = sqlx::query_scalar(queries::VERIFY_JOB_LEASE)
            .bind(&job.user_id)
            .bind(&job.job_id)
            .bind(&job.lease_token)
            .bind(now()?)
            .fetch_one(&mut *transaction)
            .await?;
        if active != 1 {
            return Err(AppError::Conflict("Task was canceled or lease expired"));
        }
        reconcile_page(
            &mut transaction,
            job,
            page,
            identity.1.as_deref(),
            report_collection_date
                .as_ref()
                .map(|collection| collection.date.as_str()),
            candidates,
        )
        .await?;
        sqlx::query(queries::INSERT_PAGE)
            .bind(&job.user_id)
            .bind(&job.file_id)
            .bind(&run_id)
            .bind(page)
            .bind(status)
            .bind(content)
            .bind(&model)
            .bind(now()?)
            .execute(&mut *transaction)
            .await?;
        sqlx::query(queries::BUMP_REPORT)
            .bind(&job.user_id)
            .bind(&job.file_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query(queries::TOUCH_USER_DATA)
            .bind(&job.user_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query(queries::INVALIDATE_EXPORTS)
            .bind(&job.user_id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        state
            .database
            .autofill_report_identity(&job.user_id, &job.file_id, identity.0, identity.1)
            .await?;
    }
    if failed {
        return Err(AppError::Invalid(
            "Some pages or fields could not be extracted",
        ));
    }
    Ok(())
}

async fn reconcile_page(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    job: &ClaimedJob,
    page: i64,
    detected_source: Option<&str>,
    document_date: Option<&str>,
    candidates: Vec<ObservationPayload>,
) -> Result<(), AppError> {
    let source: Option<String> = if job.is_rescan {
        sqlx::query_scalar(queries::RESCAN_SOURCE)
            .bind(&job.user_id)
            .bind(&job.file_id)
            .fetch_one(&mut **transaction)
            .await?
    } else {
        None
    };
    let source = source
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .or(detected_source);
    let metadata = if job.is_rescan {
        crate::lab_metric_metadata::Metadata::load(transaction, &job.user_id).await?
    } else {
        Default::default()
    };
    let key_for = |payload: &ObservationPayload, source: Option<&str>, date: Option<&str>| {
        let metric = crate::reports::learned_for_pending(payload, &metadata)
            .or_else(|| crate::laboratory::resolved_report_metric(payload, source));
        crate::report_dedup::result_key(payload, metric, date)
    };
    let mut overlaps = HashSet::new();
    if job.is_rescan
        && !candidates.is_empty()
        && let Some(source) = source.filter(|name| !name.trim().is_empty())
    {
        type Overlap = (String, String, Option<String>);
        let rows: Vec<Overlap> = sqlx::query_as(queries::RESCAN_OVERLAPS)
            .bind(&job.user_id)
            .bind(&job.file_id)
            .bind(source)
            .fetch_all(&mut **transaction)
            .await?;
        for (payload, source, date) in rows {
            let payload = serde_json::from_str(&payload)?;
            if let Some(key) = key_for(&payload, Some(&source), date.as_deref()) {
                overlaps.insert(key);
            }
        }
    }
    let stored: Vec<StoredCandidate> = sqlx::query_as(queries::REPORT_CANDIDATES)
        .bind(&job.user_id)
        .bind(&job.file_id)
        .fetch_all(&mut **transaction)
        .await?;
    let stored = stored
        .into_iter()
        .map(|row| {
            let payload: ObservationPayload = serde_json::from_str(&row.payload_json)?;
            let key = job
                .is_rescan
                .then(|| key_for(&payload, source, document_date))
                .flatten();
            Ok((row, payload, key))
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    let mut occurrences = HashMap::new();
    let mut used = HashSet::new();
    let mut scanned = HashSet::new();
    for candidate in &candidates {
        let occurrence = occurrences
            .entry((candidate.raw_name.clone(), candidate.source.quote.clone()))
            .or_insert(0);
        *occurrence += 1;
        let anchor = digest(
            format!(
                "{}|{}|{}|{}",
                page, candidate.raw_name, candidate.source.quote, occurrence
            )
            .as_bytes(),
        );
        let key = job
            .is_rescan
            .then(|| key_for(candidate, source, document_date))
            .flatten();
        if let Some(key) = &key
            && (!scanned.insert(key.clone()) || overlaps.contains(key))
        {
            continue;
        }
        let exact = stored
            .iter()
            .find(|(row, _, _)| row.candidate_key.as_ref() == Some(&anchor));
        if exact.is_some_and(|(row, _, _)| used.contains(&row.observation_id)) {
            continue;
        }
        let available = || {
            stored
                .iter()
                .filter(|(row, _, _)| !used.contains(&row.observation_id))
        };
        let existing = if job.is_rescan {
            // A protected row wins over an unedited copy of the same measurement.
            available()
                .find(|(row, _, stored_key)| !row.editable && key.is_some() && stored_key == &key)
        } else {
            None
        }
        .or(exact)
        .or_else(|| {
            let mut matches =
                available().filter(|(_, previous, _)| same_printed_result(previous, candidate));
            let first = matches.next()?;
            matches.next().is_none().then_some(first)
        })
        .or_else(|| {
            if !job.is_rescan {
                return None;
            }
            if let Some(found) =
                available().find(|(_, _, stored_key)| key.is_some() && stored_key == &key)
            {
                return Some(found);
            }
            // A changed value can still refer to the same printed row. Only replace it
            // when name, unit, date and page identify a single previous result.
            if candidates
                .iter()
                .filter(|other| same_printed_identity(other, candidate))
                .take(2)
                .count()
                != 1
            {
                return None;
            }
            let mut matches = available().filter(|(row, previous, _)| {
                row.editable && same_printed_identity(previous, candidate)
            });
            let first = matches.next()?;
            matches.next().is_none().then_some(first)
        });
        if let Some((row, previous, _)) = existing {
            used.insert(row.observation_id.clone());
            if row.editable && previous.source.page == page {
                let updated = if job.is_rescan {
                    candidate.clone()
                } else {
                    let mut updated = previous.clone();
                    if same_printed_result(previous, candidate) {
                        updated.ocr_normalization = candidate.ocr_normalization.clone();
                    }
                    updated.ocr_mapping_note = candidate.ocr_mapping_note.clone();
                    updated.ocr_metric_candidate = candidate.ocr_metric_candidate.clone();
                    updated.ocr_section_heading = candidate.ocr_section_heading.clone();
                    updated.ocr_new_metric = candidate.ocr_new_metric.clone();
                    updated
                };
                let payload = serde_json::to_string(&updated)?;
                if payload != row.payload_json {
                    sqlx::query(queries::REFRESH_OCR_CANDIDATE)
                        .bind(payload)
                        .bind(now()?)
                        .bind(&job.user_id)
                        .bind(&job.file_id)
                        .bind(&row.observation_id)
                        .execute(&mut **transaction)
                        .await?;
                }
            }
            continue;
        }
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(queries::INSERT_OBSERVATION)
            .bind(&id)
            .bind(&job.user_id)
            .bind(&job.file_id)
            .bind(&anchor)
            .execute(&mut **transaction)
            .await?;
        sqlx::query(queries::SAVE_PENDING_OBSERVATION)
            .bind(&job.user_id)
            .bind(id)
            .bind(1)
            .bind(serde_json::to_string(candidate)?)
            .bind(now()?)
            .execute(&mut **transaction)
            .await?;
    }
    for (row, _, key) in &stored {
        if row.editable
            && !used.contains(&row.observation_id)
            && key.as_ref().is_some_and(|key| scanned.contains(key))
        {
            sqlx::query(queries::DELETE_REDUNDANT_OCR_CANDIDATE)
                .bind(&job.user_id)
                .bind(&job.file_id)
                .bind(&row.observation_id)
                .execute(&mut **transaction)
                .await?;
        }
    }
    Ok(())
}

async fn extract_page(
    state: &AppState,
    job: &ClaimedJob,
    run_id: &str,
    page: i64,
    image_url: String,
    evidence: &crate::documents::PageEvidence,
    database_metrics: std::sync::Arc<crate::ocr::MetricCatalog>,
) -> Result<(ExtractedPage, String, String), AppError> {
    let _permit = state
        .inference_slots
        .acquire()
        .await
        .map_err(|_| AppError::Internal)?;
    let reply = crate::ocr::extract(
        &state.database.cpu,
        &state.config,
        page,
        image_url,
        evidence,
        database_metrics,
    )
    .await?;
    let body = reply.response_body.clone();
    let status_code = reply.status_code;
    let (model, content, output) = state
        .database
        .cpu
        .run_background(move || {
            let metadata = serde_json::from_str::<serde_json::Value>(&body).ok();
            let model = metadata
                .as_ref()
                .and_then(|value| value["model"].as_str())
                .unwrap_or("unknown")
                .to_owned();
            let content = metadata
                .as_ref()
                .and_then(|value| value["content"].as_str())
                .map(str::to_owned);
            let output = if !(200..300).contains(&status_code) {
                Err(AppError::ProviderUnavailable)
            } else {
                metadata
                    .ok_or(AppError::Invalid("OCR service returned invalid JSON"))
                    .and_then(|value| {
                        serde_json::from_value::<crate::ocr::ExtractionResponse>(value).map_err(
                            |_| AppError::Invalid("OCR service returned invalid structured output"),
                        )
                    })
            };
            Ok((model, content, output))
        })
        .await?;
    save_output(state, job, run_id, page, &model, content.as_deref(), &reply).await?;
    let result = output?;
    Ok((
        ExtractedPage {
            report_title: result.report_title,
            source_name: result.source_name,
            observations: result.observations,
            warnings: result.warnings,
        },
        result.content,
        result.model,
    ))
}

async fn save_output(
    state: &AppState,
    job: &ClaimedJob,
    run_id: &str,
    page: i64,
    model: &str,
    content: Option<&str>,
    reply: &provider::Completion,
) -> Result<(), AppError> {
    let mut transaction = state.database.pool.begin_with("BEGIN IMMEDIATE").await?;
    let active: i64 = sqlx::query_scalar(queries::VERIFY_JOB_LEASE)
        .bind(&job.user_id)
        .bind(&job.job_id)
        .bind(&job.lease_token)
        .bind(now()?)
        .fetch_one(&mut *transaction)
        .await?;
    if active != 1 {
        return Err(AppError::Conflict("Task was canceled or lease expired"));
    }
    sqlx::query(queries::INSERT_EXTRACTION_OUTPUT)
        .bind(&job.user_id)
        .bind(&job.file_id)
        .bind(run_id)
        .bind(page)
        .bind("ocr")
        .bind(&reply.response_body)
        .bind(reply.status_code as i64)
        .bind(content)
        .bind(model)
        .bind("strata")
        .bind(now()?)
        .execute(&mut *transaction)
        .await?;
    sqlx::query(queries::TOUCH_USER_DATA)
        .bind(&job.user_id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query(queries::INVALIDATE_EXPORTS)
        .bind(&job.user_id)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(())
}

async fn save_input(
    state: &AppState,
    job: &ClaimedJob,
    run_id: &str,
    page: i64,
    evidence: &crate::documents::PageEvidence,
) -> Result<(), AppError> {
    let evidence = evidence.clone();
    let input_json = state
        .database
        .cpu
        .run_background(move || Ok(serde_json::to_string(&evidence)?))
        .await?;
    let mut transaction = state.database.pool.begin_with("BEGIN IMMEDIATE").await?;
    let active: i64 = sqlx::query_scalar(queries::VERIFY_JOB_LEASE)
        .bind(&job.user_id)
        .bind(&job.job_id)
        .bind(&job.lease_token)
        .bind(now()?)
        .fetch_one(&mut *transaction)
        .await?;
    if active != 1 {
        return Err(AppError::Conflict("Task was canceled or lease expired"));
    }
    sqlx::query(queries::INSERT_EXTRACTION_INPUT)
        .bind(&job.user_id)
        .bind(&job.file_id)
        .bind(run_id)
        .bind(page)
        .bind(input_json)
        .bind(now()?)
        .execute(&mut *transaction)
        .await?;
    sqlx::query(queries::TOUCH_USER_DATA)
        .bind(&job.user_id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query(queries::INVALIDATE_EXPORTS)
        .bind(&job.user_id)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(())
}

async fn daily_view_loop(state: AppState) {
    let mut tick = tokio::time::interval(Duration::from_secs(2));
    loop {
        tick.tick().await;
        let Ok(_permit) = state.health_slots.try_acquire() else {
            continue;
        };
        let result = async { state.database.recompute_daily_next(now()?, 60).await }.await;
        if let Err(error) = result {
            tracing::warn!(code=%error,"daily view recomputation failed");
        }
    }
}
