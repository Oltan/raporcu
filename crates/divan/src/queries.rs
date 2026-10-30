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
        crate::charts::pct(self.passed, self.total)
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
    Ok(c.query_row(
        &format!("SELECT {RUN_COLS} FROM runs r JOIN projects p ON p.id = r.project_id WHERE r.id = ?1"),
        [id],
        run_row,
    )
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
    pub full_name: Option<String>,
}

impl ScenarioRow {
    /// DOORS ve Jira tag'leri ayrı rozetle gösterildiği için düz tag listesinden çıkarılır.
    pub fn plain_tags(&self) -> Vec<&String> {
        self.tags
            .iter()
            .filter(|t| common::allure::doors_id(t).is_none() && common::allure::jira_key(t).is_none())
            .collect()
    }
    /// `giris.feature:21` — aynı adlı senaryo taslağı satırlarını ayırt eder.
    pub fn location(&self) -> String {
        self.full_name
            .as_deref()
            .map(|f| f.rsplit('/').next().unwrap_or(f).to_string())
            .unwrap_or_default()
    }
    pub fn duration(&self) -> String {
        match self.duration_ms {
            Some(ms) if ms >= 1000 => format!("{:.1} sn", ms as f64 / 1000.0),
            Some(ms) => format!("{ms} ms"),
            None => "-".into(),
        }
    }
    pub fn short_message(&self) -> String {
        self.message
            .as_deref()
            .map(|m| crate::logging::truncate(m.lines().next().unwrap_or(""), 160))
            .unwrap_or_default()
    }
    pub fn is_failure(&self) -> bool {
        self.status == "failed" || self.status == "broken"
    }
}

pub const SCEN_COLS: &str = "s.id, s.run_id, s.project_id, s.branch, s.scenario_key, s.name, s.feature, s.feature_file, s.tags,
    s.doors_ids, s.jira_keys, s.status, s.message, s.duration_ms, s.attempt, s.is_latest, s.created_at, s.full_name";

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
        full_name: r.get(17)?,
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
    Ok(
        c.query_row(&format!("SELECT {SCEN_COLS} FROM scenarios s WHERE s.id = ?1"), [id], scenario_row)
            .optional()?,
    )
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
    let mut st =
        c.prepare("SELECT id, depth, name, status, message, stop_ms - start_ms FROM steps WHERE scenario_id = ?1 ORDER BY position")?;
    let rows = st.query_map([scenario_id], |r| {
        Ok(StepRow {
            id: r.get(0)?,
            depth: r.get(1)?,
            name: r.get(2)?,
            status: r.get(3)?,
            message: r.get(4)?,
            duration_ms: r.get(5)?,
        })
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
    /// Küçük metin eklerin içeriği (sayfada gösterilir)
    pub preview: Option<String>,
}

impl AttachmentRow {
    pub fn is_text(&self) -> bool {
        self.mime
            .as_deref()
            .is_some_and(|m| m.starts_with("text/") || m == "application/json")
    }
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
        preview: None,
    })
}

const ATT_COLS: &str = "id, run_id, scenario_id, step_id, origin, name, mime, source, deleted_reason";

pub fn attachments(c: &Connection, scenario_id: i64) -> Result<Vec<AttachmentRow>> {
    let mut st = c.prepare(&format!("SELECT {ATT_COLS} FROM attachments WHERE scenario_id = ?1 ORDER BY id"))?;
    let rows = st.query_map([scenario_id], att_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn attachment(c: &Connection, id: i64) -> Result<Option<AttachmentRow>> {
    Ok(
        c.query_row(&format!("SELECT {ATT_COLS} FROM attachments WHERE id = ?1"), [id], att_row)
            .optional()?,
    )
}

/// Senaryonun ilk ekran görüntüsü (rapor listesi ve OpenCode için).
pub fn first_screenshot(c: &Connection, scenario_id: i64) -> Result<Option<AttachmentRow>> {
    Ok(c.query_row(
        &format!(
            "SELECT {ATT_COLS} FROM attachments WHERE scenario_id = ?1 AND mime LIKE 'image/%' AND deleted_at IS NULL ORDER BY id LIMIT 1"
        ),
        [scenario_id],
        att_row,
    )
    .optional()?)
}

// ---------------------------------------------------------------------------------------------
// Hata bağlantıları, düzeltmeler, pipeline'lar, paylaşım

#[derive(Debug, Clone, Serialize)]
pub struct BugLink {
    pub jira_key: String,
    pub source: String,
}

pub fn bug_links(c: &Connection, scenario_id: i64) -> Result<Vec<BugLink>> {
    let mut st = c.prepare("SELECT jira_key, source FROM bug_links WHERE scenario_id = ?1 ORDER BY id")?;
    let rows = st.query_map([scenario_id], |r| {
        Ok(BugLink {
            jira_key: r.get(0)?,
            source: r.get(1)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[derive(Debug, Clone, Serialize)]
pub struct FixRow {
    pub id: i64,
    pub run_id: i64,
    pub scenario_id: Option<i64>,
    pub scenario_key: String,
    pub scenario_name: Option<String>,
    pub outcome: String,
    pub mr_iid: Option<i64>,
    pub mr_url: Option<String>,
    pub mr_state: Option<String>,
    pub branch: Option<String>,
    pub validation_passed: Option<bool>,
    pub summary: Option<String>,
    pub warnings: Vec<String>,
    pub created_at: String,
}

impl FixRow {
    pub fn outcome_label(&self) -> &'static str {
        match self.outcome.as_str() {
            "mr_opened" => "MR açıldı",
            "draft_mr" => "Draft MR (doğrulama geçmedi)",
            "no_change" => "Değişiklik yok",
            "rejected" => "Reddedildi (koruma)",
            _ => "Hata",
        }
    }
    pub fn badge(&self) -> String {
        match self.mr_iid {
            Some(i) => format!("OpenCode !{i}"),
            None => "OpenCode".into(),
        }
    }
}

const FIX_COLS: &str = "f.id, f.run_id, f.scenario_id, f.scenario_key, s.name, f.outcome, f.mr_iid, f.mr_url, f.mr_state, f.branch,
    f.validation_passed, f.summary, f.warnings, f.created_at";

fn fix_row(r: &Row) -> rusqlite::Result<FixRow> {
    let warnings: Option<String> = r.get(12)?;
    Ok(FixRow {
        id: r.get(0)?,
        run_id: r.get(1)?,
        scenario_id: r.get(2)?,
        scenario_key: r.get(3)?,
        scenario_name: r.get(4)?,
        outcome: r.get(5)?,
        mr_iid: r.get(6)?,
        mr_url: r.get(7)?,
        mr_state: r.get(8)?,
        branch: r.get(9)?,
        validation_passed: r.get::<_, Option<i64>>(10)?.map(|v| v == 1),
        summary: r.get(11)?,
        warnings: warnings.and_then(|w| serde_json::from_str(&w).ok()).unwrap_or_default(),
        created_at: r.get(13)?,
    })
}

pub fn fixes_for_run(c: &Connection, run_id: i64) -> Result<Vec<FixRow>> {
    let mut st = c.prepare(&format!(
        "SELECT {FIX_COLS} FROM fix_attempts f LEFT JOIN scenarios s ON s.id = f.scenario_id WHERE f.run_id = ?1 ORDER BY f.id"
    ))?;
    let rows = st.query_map([run_id], fix_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn fixes_for_scenario(c: &Connection, scenario_id: i64) -> Result<Vec<FixRow>> {
    let mut st = c.prepare(&format!(
        "SELECT {FIX_COLS} FROM fix_attempts f LEFT JOIN scenarios s ON s.id = f.scenario_id WHERE f.scenario_id = ?1 ORDER BY f.id"
    ))?;
    let rows = st.query_map([scenario_id], fix_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[derive(Debug, Clone, Serialize)]
pub struct PipelineRow {
    pub project_path: String,
    pub pipeline_id: String,
    pub git_ref: Option<String>,
    pub status: String,
    pub source: Option<String>,
    pub url: Option<String>,
    pub updated_at: String,
}

impl PipelineRow {
    pub fn status_label(&self) -> &'static str {
        match self.status.as_str() {
            "created" | "pending" | "waiting_for_resource" | "preparing" | "scheduled" => "kuyrukta",
            "running" => "çalışıyor",
            "success" => "bitti (başarılı)",
            "failed" => "bitti (kaldı)",
            "canceled" => "iptal",
            "skipped" => "atlandı",
            _ => "bilinmiyor",
        }
    }
    pub fn status_class(&self) -> &'static str {
        match self.status.as_str() {
            "running" => "running",
            "success" => "passed",
            "failed" => "failed",
            "canceled" | "skipped" => "skipped",
            _ => "queued",
        }
    }
}

pub fn pipelines(c: &Connection, project_id: Option<i64>, limit: i64) -> Result<Vec<PipelineRow>> {
    let mut st = c.prepare(
        "SELECT p.path, pl.pipeline_id, pl.ref, pl.status, pl.source, pl.url, pl.updated_at
         FROM pipelines pl JOIN projects p ON p.id = pl.project_id
         WHERE (?1 IS NULL OR pl.project_id = ?1) ORDER BY pl.updated_at DESC LIMIT ?2",
    )?;
    let rows = st.query_map(params![project_id, limit], |r| {
        Ok(PipelineRow {
            project_path: r.get(0)?,
            pipeline_id: r.get(1)?,
            git_ref: r.get(2)?,
            status: r.get(3)?,
            source: r.get(4)?,
            url: r.get(5)?,
            updated_at: r.get(6)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[derive(Debug, Clone, Serialize)]
pub struct ShareRow {
    pub id: i64,
    pub token: String,
    pub run_id: i64,
    pub project_path: String,
    pub branch: String,
    pub created_at: String,
    pub revoked_at: Option<String>,
}

pub fn share_links(c: &Connection, limit: i64) -> Result<Vec<ShareRow>> {
    let mut st = c.prepare(
        "SELECT sl.id, sl.token, sl.run_id, p.path, r.branch, sl.created_at, sl.revoked_at
         FROM share_links sl JOIN runs r ON r.id = sl.run_id JOIN projects p ON p.id = r.project_id
         ORDER BY sl.id DESC LIMIT ?1",
    )?;
    let rows = st.query_map([limit], |r| {
        Ok(ShareRow {
            id: r.get(0)?,
            token: r.get(1)?,
            run_id: r.get(2)?,
            project_path: r.get(3)?,
            branch: r.get(4)?,
            created_at: r.get(5)?,
            revoked_at: r.get(6)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Geçerli (iptal edilmemiş) paylaşım linkinin koşumu.
pub fn share_run(c: &Connection, token: &str) -> Result<Option<i64>> {
    Ok(c.query_row(
        "SELECT run_id FROM share_links WHERE token = ?1 AND revoked_at IS NULL",
        [token],
        |r| r.get(0),
    )
    .optional()?)
}

// ---------------------------------------------------------------------------------------------
// SW-4.7 arama

#[derive(Debug, Clone, Default, serde::Deserialize, Serialize)]
#[serde(default)]
pub struct SearchFilter {
    pub project: Option<i64>,
    pub branch: Option<String>,
    pub run: Option<i64>,
    pub tag: Option<String>,
    pub feature: Option<String>,
    pub doors: Option<String>,
    pub jira: Option<String>,
    pub triage: Option<String>,
    pub status: Option<String>,
    /// hata mesajında geçen metin
    pub q: Option<String>,
    pub page: Option<i64>,
}

fn nonempty(s: &Option<String>) -> Option<String> {
    s.as_ref().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

impl SearchFilter {
    pub fn normalized(&self) -> SearchFilter {
        SearchFilter {
            project: self.project,
            branch: nonempty(&self.branch),
            run: self.run,
            tag: nonempty(&self.tag).map(|t| t.trim_start_matches('@').to_string()),
            feature: nonempty(&self.feature),
            doors: nonempty(&self.doors).map(|d| {
                if d.to_uppercase().starts_with("DOORS-") {
                    d.to_uppercase()
                } else {
                    format!("DOORS-{d}")
                }
            }),
            jira: nonempty(&self.jira).map(|j| j.to_uppercase()),
            triage: nonempty(&self.triage),
            status: nonempty(&self.status),
            q: nonempty(&self.q),
            page: self.page,
        }
    }

    /// Sorgu dizesi (sayfalama ve CSV linkleri için), `page` hariç.
    pub fn query_string(&self) -> String {
        let mut v = Vec::new();
        let mut add = |k: &str, val: Option<String>| {
            if let Some(val) = val {
                v.push(format!("{k}={}", urlencoding::encode(&val)));
            }
        };
        add("project", self.project.map(|p| p.to_string()));
        add("branch", self.branch.clone());
        add("run", self.run.map(|p| p.to_string()));
        add("tag", self.tag.clone());
        add("feature", self.feature.clone());
        add("doors", self.doors.clone());
        add("jira", self.jira.clone());
        add("triage", self.triage.clone());
        add("status", self.status.clone());
        add("q", self.q.clone());
        v.join("&")
    }
}

fn like(s: &str) -> String {
    format!("%{}%", s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"))
}

/// WHERE parçası ve parametreleri. Yalnızca son denemeler.
pub fn search_where(f: &SearchFilter) -> (String, Vec<rusqlite::types::Value>) {
    use rusqlite::types::Value as V;
    let mut w = vec!["s.is_latest = 1".to_string()];
    let mut p: Vec<V> = Vec::new();
    if let Some(x) = f.project {
        p.push(V::Integer(x));
        w.push(format!("s.project_id = ?{}", p.len()));
    }
    if let Some(x) = &f.branch {
        p.push(V::Text(x.clone()));
        w.push(format!("s.branch = ?{}", p.len()));
    }
    if let Some(x) = f.run {
        p.push(V::Integer(x));
        w.push(format!("s.run_id = ?{}", p.len()));
    }
    if let Some(x) = &f.tag {
        p.push(V::Text(like(&format!(" {x} "))));
        w.push(format!("s.tags LIKE ?{} ESCAPE '\\'", p.len()));
    }
    if let Some(x) = &f.feature {
        p.push(V::Text(like(x)));
        w.push(format!("s.feature LIKE ?{} ESCAPE '\\'", p.len()));
    }
    if let Some(x) = &f.doors {
        p.push(V::Text(like(&format!(" {x} "))));
        w.push(format!("s.doors_ids LIKE ?{} ESCAPE '\\'", p.len()));
    }
    if let Some(x) = &f.jira {
        p.push(V::Text(like(&format!(" {x} "))));
        let a = p.len();
        p.push(V::Text(x.clone()));
        w.push(format!(
            "(s.jira_keys LIKE ?{a} ESCAPE '\\' OR EXISTS (SELECT 1 FROM bug_links b WHERE b.scenario_id = s.id AND b.jira_key = ?{}))",
            p.len()
        ));
    }
    if let Some(x) = &f.triage {
        p.push(V::Text(x.clone()));
        w.push(format!("s.triage_category = ?{}", p.len()));
    }
    if let Some(x) = &f.status {
        if x == "kaldi" {
            w.push("s.status IN ('failed', 'broken')".into());
        } else {
            p.push(V::Text(x.clone()));
            w.push(format!("s.status = ?{}", p.len()));
        }
    }
    if let Some(x) = &f.q {
        p.push(V::Text(like(x)));
        w.push(format!(
            "(s.message LIKE ?{0} ESCAPE '\\' OR s.name LIKE ?{0} ESCAPE '\\')",
            p.len()
        ));
    }
    (w.join(" AND "), p)
}

pub const PAGE_SIZE: i64 = 50;

#[derive(Debug, Clone, Serialize)]
pub struct SearchRow {
    pub s: ScenarioRow,
    pub project_path: String,
    pub triage_category: Option<String>,
}

impl SearchRow {
    pub fn triage_label(&self) -> &'static str {
        self.triage_category.as_deref().map(crate::triage::label).unwrap_or("")
    }
}

pub fn search(c: &Connection, f: &SearchFilter) -> Result<(Vec<SearchRow>, i64)> {
    let (w, p) = search_where(f);
    let total: i64 = c.query_row(
        &format!("SELECT COUNT(*) FROM scenarios s WHERE {w}"),
        rusqlite::params_from_iter(p.iter()),
        |r| r.get(0),
    )?;
    let page = f.page.unwrap_or(1).max(1);
    let mut st = c.prepare(&format!(
        "SELECT {SCEN_COLS}, p.path, s.triage_category FROM scenarios s JOIN projects p ON p.id = s.project_id WHERE {w}
         ORDER BY s.run_id DESC, s.id LIMIT {PAGE_SIZE} OFFSET {}",
        (page - 1) * PAGE_SIZE
    ))?;
    let rows = st.query_map(rusqlite::params_from_iter(p.iter()), |r| {
        Ok(SearchRow {
            s: scenario_row(r)?,
            project_path: r.get(18)?,
            triage_category: r.get(19)?,
        })
    })?;
    Ok((rows.collect::<rusqlite::Result<_>>()?, total))
}

// ---------------------------------------------------------------------------------------------
// Grafikler: gidişat ve feature bazlı özet

/// Proje + branch'in son `limit` biten koşumu, eskiden yeniye.
pub fn trend(c: &Connection, project_id: i64, branch: &str, limit: i64) -> Result<Vec<RunRow>> {
    let mut st = c.prepare(&format!(
        "SELECT * FROM (SELECT {RUN_COLS} FROM runs r JOIN projects p ON p.id = r.project_id
          WHERE r.project_id = ?1 AND r.branch = ?2 AND r.status <> 'running' AND r.total > 0
          ORDER BY r.id DESC LIMIT ?3) ORDER BY 1"
    ))?;
    let rows = st.query_map(params![project_id, branch, limit], run_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct FeatureRow {
    pub feature: String,
    pub feature_file: Option<String>,
    pub total: i64,
    pub passed: i64,
    pub failed: i64,
    pub broken: i64,
    pub other: i64,
    pub duration_ms: i64,
}

impl FeatureRow {
    pub fn failures(&self) -> i64 {
        self.failed + self.broken
    }
    pub fn pass_pct(&self) -> i64 {
        crate::charts::pct(self.passed, self.total)
    }
    pub fn duration(&self) -> String {
        let s = self.duration_ms / 1000;
        if s >= 60 {
            format!("{} dk {} sn", s / 60, s % 60)
        } else {
            format!("{s} sn")
        }
    }
    /// Durum: kaldi > broken > bilinmiyor > gecti
    pub fn state(&self) -> &'static str {
        if self.failed > 0 {
            "kaldi"
        } else if self.broken > 0 {
            "broken"
        } else if self.other > 0 && self.passed == 0 {
            "diger"
        } else {
            "gecti"
        }
    }
}

/// Bir koşumun feature'ları (son denemeler). Sorunlu olanlar önce.
pub fn run_features(c: &Connection, run_id: i64) -> Result<Vec<FeatureRow>> {
    let mut st = c.prepare(
        "SELECT COALESCE(feature, '(feature yok)'), MIN(feature_file), COUNT(*),
                SUM(status = 'passed'), SUM(status = 'failed'), SUM(status = 'broken'),
                SUM(status NOT IN ('passed', 'failed', 'broken')), COALESCE(SUM(duration_ms), 0)
         FROM scenarios WHERE run_id = ?1 AND is_latest = 1
         GROUP BY COALESCE(feature, '(feature yok)')
         ORDER BY SUM(status = 'failed') + SUM(status = 'broken') DESC, 1",
    )?;
    let rows = st.query_map([run_id], |r| {
        Ok(FeatureRow {
            feature: r.get(0)?,
            feature_file: r.get(1)?,
            total: r.get(2)?,
            passed: r.get(3)?,
            failed: r.get(4)?,
            broken: r.get(5)?,
            other: r.get(6)?,
            duration_ms: r.get(7)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Feature başına son koşumlardaki geçme oranları (eskiden yeniye), proje + branch kapsamında.
pub fn feature_trends(c: &Connection, project_id: i64, branch: &str, limit: i64) -> Result<std::collections::BTreeMap<String, Vec<f64>>> {
    let mut st = c.prepare(
        "WITH recent AS (SELECT id FROM runs WHERE project_id = ?1 AND branch = ?2 AND status <> 'running' ORDER BY id DESC LIMIT ?3)
         SELECT COALESCE(feature, '(feature yok)'), run_id, 100.0 * SUM(status = 'passed') / COUNT(*)
         FROM scenarios WHERE run_id IN (SELECT id FROM recent) AND is_latest = 1
         GROUP BY 1, run_id ORDER BY run_id",
    )?;
    let mut out: std::collections::BTreeMap<String, Vec<f64>> = Default::default();
    let mut rows = st.query(params![project_id, branch, limit])?;
    while let Some(r) = rows.next()? {
        out.entry(r.get(0)?).or_default().push(r.get(2)?);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// Feature geçmişi

/// Bir feature'ın son `limit` biten koşumdaki sayıları, eskiden yeniye.
pub fn feature_runs(c: &Connection, project_id: i64, branch: &str, feature: &str, limit: i64) -> Result<Vec<(RunRow, FeatureRow)>> {
    let runs: Vec<RunRow> = {
        let mut st = c.prepare(&format!(
            "SELECT * FROM (SELECT {RUN_COLS} FROM runs r JOIN projects p ON p.id = r.project_id
              WHERE r.project_id = ?1 AND r.branch = ?2 AND r.status <> 'running'
                AND EXISTS (SELECT 1 FROM scenarios s WHERE s.run_id = r.id AND s.is_latest = 1 AND COALESCE(s.feature, '(feature yok)') = ?3)
              ORDER BY r.id DESC LIMIT ?4) ORDER BY 1"
        ))?;
        let rows = st.query_map(params![project_id, branch, feature, limit], run_row)?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    let mut out = Vec::new();
    let mut st = c.prepare(
        "SELECT COUNT(*), SUM(status = 'passed'), SUM(status = 'failed'), SUM(status = 'broken'),
                SUM(status NOT IN ('passed', 'failed', 'broken')), COALESCE(SUM(duration_ms), 0), MIN(feature_file)
         FROM scenarios WHERE run_id = ?1 AND is_latest = 1 AND COALESCE(feature, '(feature yok)') = ?2",
    )?;
    for r in runs {
        let f = st.query_row(params![r.id, feature], |x| {
            Ok(FeatureRow {
                feature: feature.to_string(),
                total: x.get(0)?,
                passed: x.get(1)?,
                failed: x.get(2)?,
                broken: x.get(3)?,
                other: x.get(4)?,
                duration_ms: x.get(5)?,
                feature_file: x.get(6)?,
            })
        })?;
        out.push((r, f));
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize)]
pub struct MatrixCell {
    pub run_id: i64,
    /// passed | failed | broken | unknown | skipped | yok (o koşumda koşmadı)
    pub status: String,
    pub scenario_id: Option<i64>,
    pub attempts: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MatrixRow {
    pub scenario_key: String,
    pub name: String,
    pub location: String,
    pub cells: Vec<MatrixCell>,
    pub failures: i64,
    pub flaky: bool,
}

/// Senaryo × koşum matrisi: satırlar en çok kalan senaryodan başlar.
pub fn feature_matrix(c: &Connection, feature: &str, run_ids: &[i64]) -> Result<Vec<MatrixRow>> {
    if run_ids.is_empty() {
        return Ok(vec![]);
    }
    let ids = run_ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
    let mut st = c.prepare(&format!(
        "SELECT s.scenario_key, s.name, s.full_name, s.run_id, s.status, s.id,
                (SELECT COUNT(*) FROM scenarios a WHERE a.run_id = s.run_id AND a.scenario_key = s.scenario_key)
         FROM scenarios s
         WHERE s.run_id IN ({ids}) AND s.is_latest = 1 AND COALESCE(s.feature, '(feature yok)') = ?1
         ORDER BY s.run_id"
    ))?;
    let mut rows: Vec<MatrixRow> = Vec::new();
    let mut idx: std::collections::HashMap<String, usize> = Default::default();
    let mut q = st.query([feature])?;
    while let Some(r) = q.next()? {
        let key: String = r.get(0)?;
        let i = *idx.entry(key.clone()).or_insert_with(|| {
            rows.push(MatrixRow {
                scenario_key: key.clone(),
                name: String::new(),
                location: String::new(),
                cells: vec![],
                failures: 0,
                flaky: false,
            });
            rows.len() - 1
        });
        let row = &mut rows[i];
        row.name = r.get(1)?; // en yeni koşumdaki ad
        let full: Option<String> = r.get(2)?;
        row.location = full.map(|f| f.rsplit('/').next().unwrap_or(&f).to_string()).unwrap_or_default();
        row.cells.push(MatrixCell {
            run_id: r.get(3)?,
            status: r.get(4)?,
            scenario_id: Some(r.get(5)?),
            attempts: r.get(6)?,
        });
    }
    for row in rows.iter_mut() {
        let mut full = Vec::with_capacity(run_ids.len());
        for rid in run_ids {
            match row.cells.iter().find(|c| c.run_id == *rid) {
                Some(c) => full.push(c.clone()),
                None => full.push(MatrixCell {
                    run_id: *rid,
                    status: "yok".into(),
                    scenario_id: None,
                    attempts: 0,
                }),
            }
        }
        row.cells = full;
        row.failures = row.cells.iter().filter(|c| c.status == "failed" || c.status == "broken").count() as i64;
        let passed = row.cells.iter().any(|c| c.status == "passed");
        row.flaky = (passed && row.failures > 0) || row.cells.iter().any(|c| c.status == "passed" && c.attempts > 1);
    }
    rows.sort_by(|a, b| {
        b.failures
            .cmp(&a.failures)
            .then(b.flaky.cmp(&a.flaky))
            .then(a.location.cmp(&b.location))
    });
    Ok(rows)
}
