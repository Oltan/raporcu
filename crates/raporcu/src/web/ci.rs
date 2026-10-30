//! `/api/ci/*`: CI'dan gelen istekler (CI_API_TOKEN ile korunur, SW-9.1).

use crate::ingest;
use crate::upload::{self, Limits, Target, UploadError};
use crate::web::AppError;
use crate::AppState;
use axum::body::{Body, Bytes};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use common::api::{CiInfo, FinishRequest, IngestSummary, RunCreated};
use serde::Deserialize;
use std::time::Instant;

impl IntoResponse for UploadError {
    fn into_response(self) -> Response {
        (self.status, Json(serde_json::json!({ "hata": self.reason }))).into_response()
    }
}

fn limits(st: &AppState) -> Limits {
    Limits {
        max_upload: st.cfg.max_upload_bytes,
        max_entry: st.cfg.max_entry_bytes,
        max_entries: st.cfg.max_zip_entries,
    }
}

pub fn report_url(st: &AppState, run_id: i64) -> String {
    format!("{}/runs/{run_id}", st.cfg.public_url)
}

async fn handle_zip(st: AppState, body: Body, target: Target, log_ci: CiInfo) -> Result<Json<IngestSummary>, UploadError> {
    let started = Instant::now();
    let reject = |e: UploadError| {
        tracing::warn!(proje = %log_ci.project_path, branch = %log_ci.branch, neden = %e.reason, durum = e.status.as_u16(), "yükleme reddedildi");
        e
    };
    let (zip_path, size) = upload::body_to_file(body, &st.paths.tmp_dir(), st.cfg.max_upload_bytes)
        .await
        .map_err(reject)?;
    let (paths, lim, zp) = (st.paths.clone(), limits(&st), zip_path.clone());
    let res = st.db.write(move |c| Ok(upload::process_zip(c, &paths, lim, &zp, target))).await;
    let _ = tokio::fs::remove_file(&zip_path).await;
    let (mut summary, run, created) = res.map_err(UploadError::from)?.map_err(reject)?;
    summary.report_url = report_url(&st, run.run_id);
    tracing::info!(
        run_id = run.run_id,
        proje = %log_ci.project_path,
        branch = %run.branch,
        dosya_sayisi = summary.accepted_results + summary.containers + summary.attachments,
        sonuc = summary.accepted_results,
        tekrar = summary.duplicate_results,
        boyut = size,
        sure_ms = started.elapsed().as_millis() as u64,
        "yükleme alındı"
    );
    crate::hooks::after_results(&st, run.run_id).await;
    if created {
        crate::hooks::after_finish(&st, run.run_id).await;
    }
    Ok(Json(summary))
}

/// SW-2.1: POST /api/ci/runs?project_path=...&branch=... (gövde: allure-results zip)
pub async fn upload_new(State(st): State<AppState>, Query(ci): Query<CiInfo>, body: Body) -> Result<Json<IngestSummary>, UploadError> {
    if ci.project_path.trim().is_empty() {
        return Err(UploadError::new(StatusCode::BAD_REQUEST, "project_path gerekli"));
    }
    handle_zip(st, body, Target::New(Box::new(ci.clone())), ci).await
}

/// Canlı koşumun son toplu zip'i (SW-3.3: artımlı sonuçlarla tutarlı, tekrarlar sayılmaz).
pub async fn upload_existing(State(st): State<AppState>, Path(id): Path<i64>, body: Body) -> Result<Json<IngestSummary>, UploadError> {
    let ci = CiInfo {
        project_path: format!("run:{id}"),
        ..Default::default()
    };
    handle_zip(st, body, Target::Existing(id), ci).await
}

#[derive(Deserialize)]
pub struct StartBody {
    #[serde(flatten)]
    pub ci: CiInfo,
    pub total: Option<i64>,
    #[serde(default)]
    pub expected: Vec<ExpectedScenario>,
}

#[derive(Deserialize)]
pub struct ExpectedScenario {
    pub name: String,
    pub feature: Option<String>,
}

/// SW-3.1: koşum "çalışıyor" olarak açılır.
pub async fn start(State(st): State<AppState>, Json(b): Json<StartBody>) -> Result<Json<RunCreated>, AppError> {
    if b.ci.project_path.trim().is_empty() {
        return Err(AppError::bad("project_path gerekli"));
    }
    let run_id = st
        .db
        .write(move |c| {
            let tx = c.transaction()?;
            let pid = ingest::ensure_project(&tx, &b.ci)?;
            let total = b.total.or(if b.expected.is_empty() {
                None
            } else {
                Some(b.expected.len() as i64)
            });
            let run = ingest::create_run(&tx, pid, &b.ci, "running", total)?;
            for (i, e) in b.expected.iter().enumerate() {
                tx.execute(
                    "INSERT INTO expected_scenarios (run_id, position, name, feature) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![run.run_id, i as i64, e.name, e.feature],
                )?;
            }
            tx.commit()?;
            Ok(run.run_id)
        })
        .await?;
    crate::hooks::after_results(&st, run_id).await;
    Ok(Json(RunCreated {
        run_id,
        report_url: report_url(&st, run_id),
    }))
}

#[derive(Deserialize)]
pub struct ResultQuery {
    pub name: String,
}

/// SW-3.3: artımlı sonuç. Gövde tek bir allure dosyasıdır (`?name=<dosya adı>`).
pub async fn result_file(
    State(st): State<AppState>,
    Path(id): Path<i64>,
    Query(q): Query<ResultQuery>,
    body: Bytes,
) -> Result<Json<IngestSummary>, UploadError> {
    let name = q.name;
    if !common::allure::is_safe_file_name(&name) {
        return Err(UploadError::new(StatusCode::BAD_REQUEST, "geçersiz dosya adı"));
    }
    let kind = common::allure::file_kind(&name);
    let paths = st.paths.clone();
    let summary = st
        .db
        .write(move |c| {
            let Some(run) = ingest::run_ctx(c, id)? else {
                return Ok(Err(UploadError::new(StatusCode::NOT_FOUND, "koşum bulunamadı")));
            };
            let mut s = IngestSummary {
                run_id: id,
                ..Default::default()
            };
            match kind {
                common::allure::FileKind::Attachment => {
                    let dir = paths.run_files_dir(id);
                    std::fs::create_dir_all(&dir)?;
                    let dest = dir.join(&name);
                    if !dest.exists() {
                        let tmp = dir.join(format!(".{name}.part"));
                        std::fs::write(&tmp, &body)?;
                        std::fs::rename(&tmp, &dest)?;
                        s.attachments = 1;
                    }
                    c.execute(
                        "UPDATE runs SET last_activity_at = ?2 WHERE id = ?1",
                        rusqlite::params![id, crate::db::now()],
                    )?;
                }
                common::allure::FileKind::Result | common::allure::FileKind::Container => {
                    let tx = c.transaction()?;
                    if let Err(e) = upload::ingest_json(&tx, &run, &name, &body, &mut s) {
                        return Ok(Err(e));
                    }
                    ingest::link_fixture_attachments(&tx, id)?;
                    ingest::recompute_run(&tx, id)?;
                    tx.commit()?;
                }
                common::allure::FileKind::Other => {}
            }
            Ok(Ok(s))
        })
        .await
        .map_err(UploadError::from)??;
    crate::hooks::after_results(&st, id).await;
    Ok(Json(summary))
}

/// SW-3.5: koşum biter ya da yarıda kesildi olarak işaretlenir.
pub async fn finish(
    State(st): State<AppState>,
    Path(id): Path<i64>,
    body: Option<Json<FinishRequest>>,
) -> Result<Json<ingest::RunCounts>, AppError> {
    let status = body.and_then(|b| b.0.status).unwrap_or_else(|| "finished".into());
    if status != "finished" && status != "interrupted" {
        return Err(AppError::bad("status finished ya da interrupted olmalı"));
    }
    let counts = st
        .db
        .write(move |c| {
            if ingest::run_ctx(c, id)?.is_none() {
                return Ok(None);
            }
            ingest::recompute_run(c, id)?;
            Ok(Some(ingest::finish_run(c, id, &status)?))
        })
        .await?
        .ok_or_else(AppError::not_found)?;
    crate::hooks::after_finish(&st, id).await;
    Ok(Json(counts))
}

/// SW-7.1: stajci fix bu listeden OpenCode hedefli senaryoları alır.
pub async fn fix_candidates(State(st): State<AppState>, Path(id): Path<i64>) -> Result<Json<Vec<common::api::FixCandidate>>, AppError> {
    let url = st.cfg.public_url.clone();
    let exists = st.db.read(move |c| Ok(ingest::run_ctx(c, id)?.is_some())).await?;
    if !exists {
        return Err(AppError::not_found());
    }
    let paths = st.paths.clone();
    Ok(Json(st.db.read(move |c| crate::fix::candidates(c, &paths, id, &url)).await?))
}

/// SW-7.3: fix sonucu.
pub async fn fix_result(
    State(st): State<AppState>,
    Path(id): Path<i64>,
    Json(r): Json<common::api::FixResult>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conf = st.cfg.triage_auto_confidence;
    let (cfg, sablon) = (st.cfg.clone(), st.ayarlar.get().jira);
    let fid = st
        .db
        .write(move |c| {
            if ingest::run_ctx(c, id)?.is_none() {
                return Ok(None);
            }
            let fid = crate::fix::record(c, id, &r, conf)?;
            // OpenCode ürün hatası dediyse (güven eşiğin üstünde) Jira bug taslağı hazırlanır
            if let Some(t) = r.triage.as_ref().filter(|t| t.category == "urun" && t.confidence >= conf) {
                let sid: Option<i64> = c
                    .query_row(
                        "SELECT id FROM scenarios WHERE run_id = ?1 AND scenario_key = ?2 AND is_latest = 1",
                        rusqlite::params![id, r.scenario_key],
                        |x| x.get(0),
                    )
                    .ok();
                if let Some(sid) = sid {
                    if let Err(e) = crate::jira::draft_from_opencode(c, &cfg, &sablon, sid, &t.reason) {
                        tracing::warn!(run_id = id, hata = %e, "Jira bug taslağı hazırlanamadı");
                    }
                }
            }
            Ok(Some(fid))
        })
        .await?
        .ok_or_else(AppError::not_found)?;
    let _ = st.live.send(id);
    Ok(Json(serde_json::json!({ "id": fid, "report_url": report_url(&st, id) })))
}

/// SW-7.2: OpenCode isteğine eklenecek ekran görüntüsü (CI token ile).
pub async fn fix_started(State(st): State<AppState>, Path(id): Path<i64>, Json(v): Json<serde_json::Value>) -> Json<serde_json::Value> {
    tracing::info!(
        run_id = id,
        senaryo = v["scenario_key"].as_str().unwrap_or(""),
        "OpenCode denemesi başladı"
    );
    let _ = st;
    Json(serde_json::json!({ "ok": true }))
}

/// Canlı olaylar (CanliOlayEklentisi → stajci watch): şu an koşan senaryo ve adım.
/// Bellekte tutulur; koşum zaman aşımı sayacı da tazelenir.
pub async fn live_events(
    State(st): State<AppState>,
    Path(id): Path<i64>,
    Json(events): Json<Vec<serde_json::Value>>,
) -> Result<StatusCode, AppError> {
    if events.len() > 5000 {
        return Err(AppError::bad("bir seferde en fazla 5000 olay"));
    }
    let exists = st
        .db
        .write(move |c| {
            Ok(c.execute(
                "UPDATE runs SET last_activity_at = ?2 WHERE id = ?1 AND status = 'running'",
                rusqlite::params![id, crate::db::now()],
            )? > 0)
        })
        .await?;
    if !exists {
        // koşum yok ya da bitmiş: olaylar yok sayılır
        return Ok(StatusCode::ACCEPTED);
    }
    st.canli.apply(id, &events);
    let _ = st.live.send(id);
    Ok(StatusCode::NO_CONTENT)
}
