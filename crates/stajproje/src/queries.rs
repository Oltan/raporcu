//! Okuma sorguları: rapor sayfaları ve JSON API ortak kullanır. Hepsi sayfalıdır ya da tek
//! koşum/senaryo ile sınırlıdır (SW-10.6).

use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RunRow {
    pub id: i64,
    pub project_id: i64,
    pub project_path: String,
    pub project_url: Option<String>,
    pub branch: String,
    pub commit_sha: Option<String>,
    pub pipeline_id: Option<String>,
    pub pipeline_url: Option<String>,
    pub job_url: Option<String>,
    pub os: Option<String>,
    pub status: String,
    pub total_expected: Option<i64>,
    pub total: i64,
    pub passed: i64,
    pub failed: i64,
    pub broken: i64,
    pub skipped: i64,
    pub unknown: i64,
    pub started_at: String,
    pub finished_at: Option<String>,
}

impl RunRow {
    pub fn failures(&self) -> i64 {
        self.failed + self.broken
    }
    pub fn short_commit(&self) -> String {
        self.commit_sha.as_deref().map(|c| c.chars().take(8).collect()).unwrap_or_default()
    }
    pub fn commit_url(&self) -> Option<String> {
        Some(format!("{}/-/commit/{}", self.project_url.as_deref()?, self.commit_sha.as_deref()?))
    }
    pub fn progress(&self) -> String {
        match self.total_expected {
            Some(t) => format!("{}/{}", self.total, t),
            None => self.total.to_string(),
        }
    }
    pub fn pass_pct(&self) -> i64 {
        if self.total == 0 {
            0
        } else {
            self.passed * 100 / self.total
        }
    }
}

const RUN_COLS: &str = "r.id, r.project_id, p.path, p.url, r.branch, r.commit_sha, r.pipeline_id, r.pipeline_url, r.job_url, r.os,
    r.status, r.total_expected, r.total, r.passed, r.failed, r.broken, r.skipped, r.unknown, r.started_at, r.finished_at";

fn run_row(r: &Row) -> rusqlite::Result<RunRow> {
    Ok(RunRow {
        id: r.get(0)?,
        project_id: r.get(1)?,
        project_path: r.get(2)?,
        project_url: r.get(3)?,
        branch: r.get(4)?,
        commit_sha: r.get(5)?,
        pipeline_id: r.get(6)?,
        pipeline_url: r.get(7)?,
        job_url: r.get(8)?,
        os: r.get(9)?,
        status: r.get(10)?,
        total_expected: r.get(11)?,
        total: r.get(12)?,
        passed: r.get(13)?,
        failed: r.get(14)?,
        broken: r.get(15)?,
        skipped: r.get(16)?,
        unknown: r.get(17)?,
        started_at: r.get(18)?,
        finished_at: r.get(19)?,
    })
}

pub fn run(c: &Connection, id: i64) -> Result<Option<RunRow>> {
    Ok(c
        .query_row(&format!("SELECT {RUN_COLS} FROM runs r JOIN projects p ON p.id = r.project_id WHERE r.id = ?1"), [id], run_row)
        .optional()?)
}

pub fn list_runs(c: &Connection, project_id: Option<i64>, branch: Option<&str>, limit: i64, offset: i64) -> Result<Vec<RunRow>> {
    let mut st = c.prepare(&format!(
        "SELECT {RUN_COLS} FROM runs r JOIN projects p ON p.id = r.project_id
         WHERE (?1 IS NULL OR r.project_id = ?1) AND (?2 IS NULL OR r.branch = ?2)
         ORDER BY r.id DESC LIMIT ?3 OFFSET ?4"
    ))?;
    let rows = st.query_map(params![project_id, branch, limit, offset], run_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[derive(Debug, Clone, Serialize)]
pub struct Project {
    pub id: i64,
    pub path: String,
    pub branches: Vec<String>,
}

pub fn projects(c: &Connection) -> Result<Vec<Project>> {
    let mut st = c.prepare("SELECT id, path FROM projects ORDER BY path")?;
    let list: Vec<(i64, String)> = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
    let mut out = Vec::new();
    for (id, path) in list {
        let mut b = c.prepare("SELECT DISTINCT branch FROM runs WHERE project_id = ?1 ORDER BY branch")?;
        let branches = b.query_map([id], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
        out.push(Project { id, path, branches });
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize)]
pub struct ScenarioRow {
    pub id: i64,
    pub run_id: i64,
    pub project_id: i64,
    pub branch: String,
    pub scenario_key: String,
    pub name: String,
    pub feature: Option<String>,
    pub feature_file: Option<String>,
    pub tags: Vec<String>,
    pub doors_ids: Vec<String>,
    pub jira_keys: Vec<String>,
    pub status: String,
    pub message: Option<String>,
    pub duration_ms: Option<i64>,
    pub attempt: i64,
    pub is_latest: bool,
    pub created_at: String,
}

impl ScenarioRow {
    pub fn duration(&self) -> String {
        match self.duration_ms {
            Some(ms) if ms >= 1000 => format!("{:.1} sn", ms as f64 / 1000.0),
            Some(ms) => format!("{ms} ms"),
            None => "-".into(),
        }
    }
    pub fn short_message(&self) -> String {
        self.message.as_deref().map(|m| crate::logging::truncate(m.lines().next().unwrap_or(""), 160)).unwrap_or_default()
    }
    pub fn is_failure(&self) -> bool {
        self.status == "failed" || self.status == "broken"
    }
}

pub const SCEN_COLS: &str = "s.id, s.run_id, s.project_id, s.branch, s.scenario_key, s.name, s.feature, s.feature_file, s.tags,
    s.doors_ids, s.jira_keys, s.status, s.message, s.duration_ms, s.attempt, s.is_latest, s.created_at";

fn split(s: String) -> Vec<String> {
    s.split_whitespace().map(String::from).collect()
}

pub fn scenario_row(r: &Row) -> rusqlite::Result<ScenarioRow> {
    Ok(ScenarioRow {
        id: r.get(0)?,
        run_id: r.get(1)?,
        project_id: r.get(2)?,
        branch: r.get(3)?,
        scenario_key: r.get(4)?,
        name: r.get(5)?,
        feature: r.get(6)?,
        feature_file: r.get(7)?,
        tags: split(r.get(8)?),
        doors_ids: split(r.get(9)?),
        jira_keys: split(r.get(10)?),
        status: r.get(11)?,
        message: r.get(12)?,
        duration_ms: r.get(13)?,
        attempt: r.get(14)?,
        is_latest: r.get::<_, i64>(15)? == 1,
        created_at: r.get(16)?,
    })
}

/// Koşumun senaryoları (son denemeler). Başarısızlar önce.
pub fn run_scenarios(c: &Connection, run_id: i64) -> Result<Vec<ScenarioRow>> {
    let mut st = c.prepare(&format!(
        "SELECT {SCEN_COLS} FROM scenarios s WHERE s.run_id = ?1 AND s.is_latest = 1
         ORDER BY CASE s.status WHEN 'failed' THEN 0 WHEN 'broken' THEN 1 WHEN 'unknown' THEN 2 WHEN 'skipped' THEN 3 ELSE 4 END,
                  s.feature, COALESCE(s.start_ms, 0), s.id"
    ))?;
    let rows = st.query_map([run_id], scenario_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn scenario(c: &Connection, id: i64) -> Result<Option<ScenarioRow>> {
    Ok(c.query_row(&format!("SELECT {SCEN_COLS} FROM scenarios s WHERE s.id = ?1"), [id], scenario_row).optional()?)
}

/// SW-4.9: aynı koşumdaki tüm denemeler, sırayla.
pub fn attempts(c: &Connection, run_id: i64, key: &str) -> Result<Vec<ScenarioRow>> {
    let mut st = c.prepare(&format!(
        "SELECT {SCEN_COLS} FROM scenarios s WHERE s.run_id = ?1 AND s.scenario_key = ?2 ORDER BY s.attempt"
    ))?;
    let rows = st.query_map(params![run_id, key], scenario_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[derive(Debug, Clone, Serialize)]
pub struct StepRow {
    pub id: i64,
    pub depth: i64,
    pub name: String,
    pub status: String,
    pub message: Option<String>,
    pub duration_ms: Option<i64>,
}

pub fn steps(c: &Connection, scenario_id: i64) -> Result<Vec<StepRow>> {
    let mut st = c.prepare(
        "SELECT id, depth, name, status, message, stop_ms - start_ms FROM steps WHERE scenario_id = ?1 ORDER BY position",
    )?;
    let rows = st.query_map([scenario_id], |r| {
        Ok(StepRow { id: r.get(0)?, depth: r.get(1)?, name: r.get(2)?, status: r.get(3)?, message: r.get(4)?, duration_ms: r.get(5)? })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[derive(Debug, Clone, Serialize)]
pub struct AttachmentRow {
    pub id: i64,
    pub run_id: i64,
    pub scenario_id: Option<i64>,
    pub step_id: Option<i64>,
    pub origin: String,
    pub name: Option<String>,
    pub mime: Option<String>,
    pub source: String,
    pub deleted_reason: Option<String>,
}

impl AttachmentRow {
    pub fn is_image(&self) -> bool {
        self.mime.as_deref().is_some_and(|m| m.starts_with("image/"))
    }
    pub fn is_video(&self) -> bool {
        self.mime.as_deref().is_some_and(|m| m.starts_with("video/"))
    }
    pub fn is_deleted(&self) -> bool {
        self.deleted_reason.is_some()
    }
    pub fn title(&self) -> String {
        self.name.clone().unwrap_or_else(|| self.source.clone())
    }
}

fn att_row(r: &Row) -> rusqlite::Result<AttachmentRow> {
    Ok(AttachmentRow {
        id: r.get(0)?,
        run_id: r.get(1)?,
        scenario_id: r.get(2)?,
        step_id: r.get(3)?,
        origin: r.get(4)?,
        name: r.get(5)?,
        mime: r.get(6)?,
        source: r.get(7)?,
        deleted_reason: r.get(8)?,
    })
}

const ATT_COLS: &str = "id, run_id, scenario_id, step_id, origin, name, mime, source, deleted_reason";

pub fn attachments(c: &Connection, scenario_id: i64) -> Result<Vec<AttachmentRow>> {
    let mut st = c.prepare(&format!("SELECT {ATT_COLS} FROM attachments WHERE scenario_id = ?1 ORDER BY id"))?;
    let rows = st.query_map([scenario_id], att_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn attachment(c: &Connection, id: i64) -> Result<Option<AttachmentRow>> {
    Ok(c.query_row(&format!("SELECT {ATT_COLS} FROM attachments WHERE id = ?1"), [id], att_row).optional()?)
}

/// Senaryonun ilk ekran görüntüsü (rapor listesi ve OpenCode için).
pub fn first_screenshot(c: &Connection, scenario_id: i64) -> Result<Option<AttachmentRow>> {
    Ok(c
        .query_row(
            &format!(
                "SELECT {ATT_COLS} FROM attachments WHERE scenario_id = ?1 AND mime LIKE 'image/%' AND deleted_at IS NULL ORDER BY id LIMIT 1"
            ),
            [scenario_id],
            att_row,
        )
        .optional()?)
}
