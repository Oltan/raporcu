//! GitLab: webhook alma (SW-3.6, SW-9.3, SW-7.5) ve API ile fix işini tetikleme (SW-7.4).
//! Token yalnızca yapılandırmadan okunur, hata mesajlarına ve loga yazılmaz (SW-9.2).

use crate::config::redact;
use crate::db::now;
use crate::AppState;
use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use std::io::Read;

/// Pipeline Hook: pipelines tablosunu günceller. Gece (schedule) pipeline'ı bittiyse true.
pub fn apply_pipeline_event(c: &Connection, v: &Value) -> Result<Option<(i64, String, String, Option<String>)>> {
    let attrs = &v["object_attributes"];
    let project = &v["project"];
    let pid = attrs["id"]
        .as_i64()
        .map(|i| i.to_string())
        .or_else(|| attrs["id"].as_str().map(String::from));
    let Some(pipeline_id) = pid else { bail!("pipeline id yok") };
    let status = attrs["status"].as_str().unwrap_or("unknown").to_string();
    let ci = common::api::CiInfo {
        project_id: project["id"].as_i64().map(|i| i.to_string()),
        project_path: project["path_with_namespace"].as_str().unwrap_or("").to_string(),
        project_url: project["web_url"].as_str().map(String::from),
        ..Default::default()
    };
    let project_id = crate::ingest::ensure_project(c, &ci)?;
    let url = attrs["url"]
        .as_str()
        .map(String::from)
        .or_else(|| ci.project_url.as_ref().map(|u| format!("{u}/-/pipelines/{pipeline_id}")));
    let source = attrs["source"].as_str().map(String::from);
    c.execute(
        "INSERT INTO pipelines (project_id, pipeline_id, ref, status, source, url, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(project_id, pipeline_id) DO UPDATE SET status = excluded.status, ref = COALESCE(excluded.ref, ref),
             source = COALESCE(excluded.source, source), url = COALESCE(excluded.url, url), updated_at = excluded.updated_at",
        params![project_id, pipeline_id, attrs["ref"].as_str(), status, source, url, now()],
    )?;
    tracing::info!(proje = %ci.project_path, pipeline_id = %pipeline_id, durum = %status, "webhook alındı");
    Ok(Some((project_id, pipeline_id, status, source)))
}

/// Merge Request Hook: auto-fix MR'ının durumu (SW-7.5: birleştirilen / reddedilen).
pub fn apply_mr_event(c: &Connection, v: &Value) -> Result<usize> {
    let attrs = &v["object_attributes"];
    let Some(iid) = attrs["iid"].as_i64() else { return Ok(0) };
    let state = match attrs["state"].as_str().unwrap_or("") {
        "merged" => "merged",
        "closed" => "closed",
        _ => "opened",
    };
    let project_gid = v["project"]["id"].as_i64().map(|i| i.to_string());
    let path = v["project"]["path_with_namespace"].as_str().unwrap_or("");
    let project_id: Option<i64> = c
        .query_row(
            "SELECT id FROM projects WHERE gitlab_id = ?1 OR path = ?2 LIMIT 1",
            params![project_gid, path],
            |r| r.get(0),
        )
        .optional()?;
    let Some(pid) = project_id else { return Ok(0) };
    let n = c.execute(
        "UPDATE fix_attempts SET mr_state = ?3, updated_at = ?4 WHERE project_id = ?1 AND mr_iid = ?2",
        params![pid, iid, state, now()],
    )?;
    if n > 0 {
        tracing::info!(proje = path, mr = iid, durum = state, "webhook alındı");
    }
    Ok(n)
}

pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .build()
        .into()
}

/// SW-7.4: yalnızca fix işini çalıştıran pipeline tetikler.
pub fn trigger_fix_pipeline(st: &AppState, project_gitlab_id: &str, git_ref: &str, run_id: i64) -> Result<Value> {
    let (Some(base), Some(token)) = (&st.cfg.gitlab_url, &st.cfg.gitlab_token) else {
        bail!("GitLab bağlantısı ayarlı değil (GITLAB_URL, GITLAB_TOKEN)");
    };
    let url = format!("{base}/api/v4/projects/{}/pipeline", urlencoding::encode(project_gitlab_id));
    let body = serde_json::json!({
        "ref": git_ref,
        "variables": [
            { "key": "STAJ_FIX_ONLY", "value": "true" },
            { "key": "STAJ_RUN_ID", "value": run_id.to_string() },
        ]
    });
    let secrets = st.cfg.secrets();
    let mut resp = agent()
        .post(&url)
        .header("PRIVATE-TOKEN", token.expose())
        .header("Content-Type", "application/json")
        .send(body.to_string())
        .map_err(|e| anyhow::anyhow!("GitLab'a ulaşılamadı: {}", redact(&e.to_string(), &secrets)))?;
    let status = resp.status().as_u16();
    let mut text = String::new();
    resp.body_mut().as_reader().take(1 << 20).read_to_string(&mut text)?;
    if !(200..300).contains(&status) {
        tracing::warn!(servis = "gitlab", durum = status, deneme = 1, "GitLab pipeline tetiklenemedi");
        bail!("GitLab {status}: {}", redact(&crate::logging::truncate(&text, 300), &secrets));
    }
    Ok(serde_json::from_str(&text).unwrap_or(Value::Null))
}
