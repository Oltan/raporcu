//! HTML sayfaları (askama + htmx). Raporları görmek giriş gerektirmez (kapalı ağ).

use crate::auth::{self, Admin};
use crate::queries::{self, AttachmentRow, BugLink, FixRow, Project, RunRow, ScenarioRow, SearchFilter, SearchRow, StepRow};
use crate::triage::{self, Decision};
use crate::web::{render, AppError};
use crate::AppState;
use askama::Template;
use axum::extract::{Path, Query, RawQuery, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Form;
use serde::Deserialize;
use std::collections::HashSet;

pub mod filters {
    #[askama::filter_fn]
    pub fn zaman(s: impl std::fmt::Display, _env: &dyn askama::Values) -> askama::Result<String> {
        let s = s.to_string();
        Ok(match chrono::DateTime::parse_from_rfc3339(&s) {
            Ok(t) => t.with_timezone(&chrono::Local).format("%d.%m.%Y %H:%M").to_string(),
            Err(_) => s,
        })
    }

    #[askama::filter_fn]
    pub fn enc(s: impl std::fmt::Display, _env: &dyn askama::Values) -> askama::Result<String> {
        Ok(urlencoding::encode(&s.to_string()).into_owned())
    }
}

/// Her sayfanın ortak bağlamı.
pub struct Layout {
    pub admin: bool,
    pub banner: Option<String>,
    /// Paylaşım görünümünde token; linkler buna göre kurulur.
    pub share: Option<String>,
}

impl Layout {
    pub fn file_url(&self, id: impl std::borrow::Borrow<i64>) -> String {
        let id = *id.borrow();
        match &self.share {
            Some(t) => format!("/s/{t}/files/{id}"),
            None => format!("/files/{id}"),
        }
    }
    pub fn scenario_url(&self, id: impl std::borrow::Borrow<i64>) -> String {
        let id = *id.borrow();
        match &self.share {
            Some(t) => format!("/s/{t}/scenarios/{id}"),
            None => format!("/scenarios/{id}"),
        }
    }
    pub fn run_url(&self, id: impl std::borrow::Borrow<i64>) -> String {
        let id = *id.borrow();
        match &self.share {
            Some(t) => format!("/s/{t}"),
            None => format!("/runs/{id}"),
        }
    }
}

pub fn layout(st: &AppState, headers: &HeaderMap) -> Layout {
    let admin = auth::cookie_value(headers, auth::SESSION_COOKIE).is_some_and(|t| st.sessions.valid(&t));
    Layout {
        admin,
        banner: if admin { st.disk.warning_text() } else { None },
        share: None,
    }
}

fn message(l: Layout, status: StatusCode, title: &str, text: &str) -> Response {
    #[derive(Template)]
    #[template(path = "message.html")]
    struct M<'a> {
        l: Layout,
        title: &'a str,
        text: &'a str,
    }
    (status, render(&M { l, title, text })).into_response()
}

// ---------------------------------------------------------------------------------------------
// Dashboard (SW-4.2, SW-4.6, SW-3.6)

#[derive(Deserialize, Default)]
pub struct DashQuery {
    project: Option<String>,
    branch: Option<String>,
    page: Option<i64>,
}

#[derive(Template)]
#[template(path = "dashboard.html")]
struct Dashboard {
    l: Layout,
    projects: Vec<Project>,
    branches: Vec<String>,
    project: Option<i64>,
    branch: Option<String>,
    runs: Vec<RunRow>,
    pipelines: Vec<queries::PipelineRow>,
    flaky: Vec<ScenarioRow>,
    window: usize,
    page: i64,
    filter_qs: String,
    charts: Option<DashCharts>,
}

/// Dashboard grafikleri: kapsam (proje + branch) için son biten koşum ve gidişat.
pub struct DashCharts {
    pub scope: String,
    pub last: RunRow,
    pub donut: String,
    pub legend: Vec<LegendItem>,
    pub line: String,
    pub columns: String,
    pub trend_n: usize,
    pub features: Vec<DashFeature>,
}

pub struct DashFeature {
    pub anchor: String,
    pub f: queries::FeatureRow,
    pub bar: String,
    pub spark: String,
    pub last_pct: i64,
    pub first_pct: i64,
}

impl DashFeature {
    /// Gidişat yönü: son koşum, penceredeki ilk koşuma göre
    pub fn change(&self) -> i64 {
        self.last_pct - self.first_pct
    }
}

const TREND_RUNS: i64 = 20;
const FEATURE_TREND_RUNS: i64 = 10;

fn dash_charts(c: &rusqlite::Connection, project_id: i64, branch: &str) -> anyhow::Result<Option<DashCharts>> {
    let trend = queries::trend(c, project_id, branch, TREND_RUNS)?;
    let Some(last) = trend.last().cloned() else { return Ok(None) };
    let fmt_day = |s: &str| {
        chrono::DateTime::parse_from_rfc3339(s)
            .map(|t| t.with_timezone(&chrono::Local).format("%d.%m").to_string())
            .unwrap_or_default()
    };
    let points: Vec<crate::charts::Point> = trend
        .iter()
        .map(|r| crate::charts::Point {
            label: format!("#{}", r.id),
            href: format!("/runs/{}", r.id),
            value: crate::charts::pct(r.passed, r.total) as f64,
            detail: format!(
                "{} · {} senaryo, {} geçti, {} kaldı",
                fmt_day(&r.started_at),
                r.total,
                r.passed,
                r.failures()
            ),
        })
        .collect();
    let columns: Vec<crate::charts::Column> = trend
        .iter()
        .map(|r| crate::charts::Column {
            label: format!("#{}", r.id),
            href: Some(format!("/runs/{}", r.id)),
            title: format!("Koşum #{} ({})", r.id, fmt_day(&r.started_at)),
            parts: run_slices(r),
        })
        .collect();
    let ftrends = queries::feature_trends(c, project_id, branch, FEATURE_TREND_RUNS)?;
    let features = queries::run_features(c, last.id)?
        .into_iter()
        .map(|f| {
            let vals = ftrends.get(&f.feature).cloned().unwrap_or_default();
            let bar = crate::charts::thin_bar(&crate::charts::status_slices(f.passed, f.failed, f.broken, f.other));
            DashFeature {
                anchor: feature_anchor(&f.feature),
                spark: crate::charts::sparkline(&vals, &format!("{} geçme oranı gidişatı", f.feature)),
                last_pct: vals.last().copied().unwrap_or(0.0).round() as i64,
                first_pct: vals.first().copied().unwrap_or(0.0).round() as i64,
                bar,
                f,
            }
        })
        .collect();
    Ok(Some(DashCharts {
        scope: format!("{} · {}", last.project_path, last.branch),
        donut: run_donut(&last),
        legend: legend(&run_slices(&last)),
        line: crate::charts::line_pct(&points, "Koşum başına geçme oranı"),
        columns: crate::charts::stacked_columns(&columns, "Koşum başına senaryo durumları"),
        trend_n: trend.len(),
        features,
        last,
    }))
}

impl Dashboard {
    fn project_selected(&self, id: &i64) -> bool {
        self.project == Some(*id)
    }
}

pub async fn dashboard(State(st): State<AppState>, headers: HeaderMap, Query(q): Query<DashQuery>) -> Result<Response, AppError> {
    let l = layout(&st, &headers);
    let project = q.project.as_deref().and_then(|p| p.parse::<i64>().ok());
    let branch = q.branch.filter(|b| !b.is_empty());
    let page = q.page.unwrap_or(1).max(1);
    let window = st.cfg.flaky_window;
    let (b2, p2) = (branch.clone(), project);
    let (projects, runs, pipelines, flaky, charts) = st
        .db
        .read(move |c| {
            let projects = queries::projects(c)?;
            let runs = queries::list_runs(c, p2, b2.as_deref(), 50, (page - 1) * 50)?;
            let pipelines = queries::pipelines(c, p2, 10)?;
            // flaky: seçili proje/branch, yoksa son koşumun proje/branch'i
            let mut flaky = Vec::new();
            let scope = match (p2, &b2) {
                (Some(p), Some(b)) => Some((p, b.clone())),
                _ => runs
                    .iter()
                    .find(|r| r.status != "running")
                    .or(runs.first())
                    .map(|r| (r.project_id, r.branch.clone())),
            };
            let mut charts = None;
            if let Some((p, b)) = scope {
                let keys = crate::history::flaky_keys(c, p, &b, window)?;
                for k in keys.iter().take(50) {
                    if let Some(s) = crate::history::scenario_history(c, p, Some(&b), k, 1, 0)?.into_iter().next() {
                        flaky.push(s);
                    }
                }
                charts = dash_charts(c, p, &b)?;
            }
            Ok((projects, runs, pipelines, flaky, charts))
        })
        .await?;
    let branches = projects
        .iter()
        .filter(|p| project.is_none_or(|id| id == p.id))
        .flat_map(|p| p.branches.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut filter_qs = String::new();
    if let Some(p) = project {
        filter_qs.push_str(&format!("project={p}"));
    }
    if let Some(b) = &branch {
        filter_qs.push_str(&format!("&branch={}", urlencoding::encode(b)));
    }
    Ok(render(&Dashboard {
        l,
        projects,
        branches,
        project,
        branch,
        runs,
        pipelines,
        flaky,
        window,
        page,
        filter_qs,
        charts,
    }))
}

// ---------------------------------------------------------------------------------------------
// Koşum raporu (SW-4.1, 4.3, 4.4, 4.9)

pub struct RunScenario {
    pub s: ScenarioRow,
    pub attempts: i64,
    pub triage: Option<Decision>,
    pub jira: Vec<String>,
    pub fix: Option<FixRow>,
    pub shot: Option<i64>,
    pub flaky: bool,
}

#[derive(Template)]
#[template(path = "run.html")]
struct RunPage {
    l: Layout,
    run: RunRow,
    groups: Vec<FeatureGroup>,
    donut: String,
    legend: Vec<LegendItem>,
    fixes: Vec<FixRow>,
    notice: Option<String>,
}

/// Koşum raporunda bir feature: özet satırı + senaryoları (SW-4.1: koşum → feature → senaryo → adım).
pub struct FeatureGroup {
    pub f: queries::FeatureRow,
    pub bar: String,
    pub rows: Vec<RunScenario>,
    pub anchor: String,
}

/// Grafik lejantı: renk sınıfı, ad, sayı, yüzde (değerler metinde, renk yalnızca işarette).
pub struct LegendItem {
    pub class: &'static str,
    pub name: &'static str,
    pub value: i64,
    pub pct: i64,
}

/// Feature için sayfa içi bağlantı adı (`#f-giris-ekrani`).
pub fn feature_anchor(name: &str) -> String {
    let mut out = String::from("f-");
    for ch in name.to_lowercase().chars() {
        let c = match ch {
            'ç' => 'c',
            'ğ' => 'g',
            'ı' | 'i' => 'i',
            'ö' => 'o',
            'ş' => 's',
            'ü' => 'u',
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        };
        if !(c == '-' && out.ends_with('-')) {
            out.push(c);
        }
    }
    out.trim_end_matches('-').to_string()
}

pub fn legend(slices: &[crate::charts::Slice]) -> Vec<LegendItem> {
    let total: i64 = slices.iter().map(|s| s.value).sum();
    slices
        .iter()
        .map(|s| LegendItem {
            class: s.class,
            name: s.name,
            value: s.value,
            pct: crate::charts::pct(s.value, total),
        })
        .collect()
}

pub fn run_slices(run: &RunRow) -> Vec<crate::charts::Slice> {
    crate::charts::status_slices(run.passed, run.failed, run.broken, run.unknown + run.skipped)
}

pub fn run_donut(run: &RunRow) -> String {
    let center = if run.status == "running" {
        run.progress()
    } else {
        run.total.to_string()
    };
    crate::charts::donut(
        &run_slices(run),
        &center,
        &format!("%{} geçti", run.pass_pct()),
        &format!("Koşum #{} durum dağılımı", run.id),
    )
}

/// Senaryoları feature'a göre gruplar; sorunlu feature'lar önce.
pub fn group_by_feature(rows: Vec<RunScenario>) -> Vec<FeatureGroup> {
    let mut order: Vec<String> = Vec::new();
    let mut map: std::collections::HashMap<String, Vec<RunScenario>> = Default::default();
    for r in rows {
        let f = r.s.feature.clone().unwrap_or_else(|| "(feature yok)".into());
        if !map.contains_key(&f) {
            order.push(f.clone());
        }
        map.entry(f).or_default().push(r);
    }
    let mut groups: Vec<FeatureGroup> = order
        .into_iter()
        .map(|name| {
            let rows = map.remove(&name).unwrap_or_default();
            let mut f = queries::FeatureRow {
                feature: name,
                ..Default::default()
            };
            for r in &rows {
                f.total += 1;
                f.duration_ms += r.s.duration_ms.unwrap_or(0);
                f.feature_file = f.feature_file.take().or_else(|| r.s.feature_file.clone());
                match r.s.status.as_str() {
                    "passed" => f.passed += 1,
                    "failed" => f.failed += 1,
                    "broken" => f.broken += 1,
                    _ => f.other += 1,
                }
            }
            let bar = crate::charts::thin_bar(&crate::charts::status_slices(f.passed, f.failed, f.broken, f.other));
            FeatureGroup {
                anchor: feature_anchor(&f.feature),
                f,
                bar,
                rows,
            }
        })
        .collect();
    groups.sort_by(|a, b| b.f.failures().cmp(&a.f.failures()).then(a.f.feature.cmp(&b.f.feature)));
    groups
}

impl RunPage {
    fn new(l: Layout, run: RunRow, rows: Vec<RunScenario>, fixes: Vec<FixRow>, notice: Option<String>) -> RunPage {
        RunPage {
            donut: run_donut(&run),
            legend: legend(&run_slices(&run)),
            groups: group_by_feature(rows),
            l,
            run,
            fixes,
            notice,
        }
    }
}

pub fn load_run(
    c: &rusqlite::Connection,
    id: i64,
    auto_conf: f64,
    window: usize,
) -> anyhow::Result<Option<(RunRow, Vec<RunScenario>, Vec<FixRow>)>> {
    let Some(run) = queries::run(c, id)? else { return Ok(None) };
    let flaky: HashSet<String> = crate::history::flaky_keys(c, run.project_id, &run.branch, window)?;
    let fixes = queries::fixes_for_run(c, id)?;
    let mut counts = std::collections::HashMap::new();
    {
        let mut st = c.prepare("SELECT scenario_key, COUNT(*) FROM scenarios WHERE run_id = ?1 GROUP BY scenario_key")?;
        let mut rows = st.query([id])?;
        while let Some(r) = rows.next()? {
            counts.insert(r.get::<_, String>(0)?, r.get::<_, i64>(1)?);
        }
    }
    let mut rows = Vec::new();
    for s in queries::run_scenarios(c, id)? {
        let triage = if s.is_failure() {
            triage::current(c, s.id, auto_conf)?
        } else {
            None
        };
        let mut jira = s.jira_keys.clone();
        for b in queries::bug_links(c, s.id)? {
            if !jira.contains(&b.jira_key) {
                jira.push(b.jira_key);
            }
        }
        let fix = fixes.iter().rev().find(|f| f.scenario_key == s.scenario_key).cloned();
        let shot = if s.is_failure() {
            queries::first_screenshot(c, s.id)?.map(|a| a.id)
        } else {
            None
        };
        rows.push(RunScenario {
            attempts: *counts.get(&s.scenario_key).unwrap_or(&1),
            flaky: flaky.contains(&s.scenario_key),
            triage,
            jira,
            fix,
            shot,
            s,
        });
    }
    Ok(Some((run, rows, fixes)))
}

#[derive(Deserialize, Default)]
pub struct NoticeQuery {
    notice: Option<String>,
}

pub async fn run_page(
    State(st): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Query(nq): Query<NoticeQuery>,
) -> Result<Response, AppError> {
    let l = layout(&st, &headers);
    let (conf, window) = (st.cfg.triage_auto_confidence, st.cfg.flaky_window);
    let Some((run, rows, fixes)) = st.db.read(move |c| load_run(c, id, conf, window)).await? else {
        return Ok(message(l, StatusCode::NOT_FOUND, "Koşum bulunamadı", "Bu numarada bir koşum yok."));
    };
    let notice = match nq.notice.as_deref() {
        Some("fix") => Some("OpenCode fix işi GitLab'da tetiklendi. Sonuç bu rapora yazılacak.".to_string()),
        Some(t) if t.starts_with("share:") => {
            let tok = &t[6..];
            let url = format!("{}/s/{}", st.cfg.public_url, html_escape(tok));
            Some(format!("Paylaşım linki: <a href=\"{url}\">{url}</a>"))
        }
        _ => None,
    };
    Ok(render(&RunPage::new(l, run, rows, fixes, notice)))
}

fn html_escape(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

// ---------------------------------------------------------------------------------------------
// Senaryo detayı (SW-4.1, 4.5, 4.9, 6.1)

#[derive(Template)]
#[template(path = "scenario.html")]
struct ScenarioPage {
    l: Layout,
    s: ScenarioRow,
    message: Option<String>,
    trace: Option<String>,
    steps: Vec<StepRow>,
    atts: Vec<AttachmentRow>,
    attempts: Vec<ScenarioRow>,
    triage: Option<Decision>,
    triage_history: Vec<Decision>,
    categories: &'static [(&'static str, &'static str)],
    bugs: Vec<BugLink>,
    jira: Vec<String>,
    jira_url: Option<String>,
    fixes: Vec<FixRow>,
}

#[allow(clippy::type_complexity)]
fn load_scenario(c: &rusqlite::Connection, paths: &crate::Paths, id: i64, conf: f64) -> anyhow::Result<Option<ScenarioPage>> {
    let Some(s) = queries::scenario(c, id)? else { return Ok(None) };
    let trace: Option<String> = c.query_row("SELECT trace FROM scenarios WHERE id = ?1", [id], |r| r.get(0))?;
    let bugs = queries::bug_links(c, id)?;
    let mut jira = s.jira_keys.clone();
    for b in &bugs {
        if !jira.contains(&b.jira_key) {
            jira.push(b.jira_key.clone());
        }
    }
    Ok(Some(ScenarioPage {
        l: Layout {
            admin: false,
            banner: None,
            share: None,
        },
        message: s.message.clone(),
        trace,
        steps: queries::steps(c, id)?,
        atts: queries::attachments(c, id)?
            .into_iter()
            .map(|mut a| {
                if a.is_text() && !a.is_deleted() {
                    let p = paths.attachment(a.run_id, &a.source);
                    if std::fs::metadata(&p).is_ok_and(|m| m.len() <= 8192) {
                        a.preview = std::fs::read(&p).ok().map(|b| String::from_utf8_lossy(&b).into_owned());
                    }
                }
                a
            })
            .collect(),
        attempts: queries::attempts(c, s.run_id, &s.scenario_key)?,
        triage: triage::current(c, id, conf)?,
        triage_history: triage::history(c, id)?,
        categories: triage::CATEGORIES,
        bugs,
        jira,
        jira_url: None,
        fixes: queries::fixes_for_scenario(c, id)?,
        s,
    }))
}

pub async fn scenario_page(State(st): State<AppState>, headers: HeaderMap, Path(id): Path<i64>) -> Result<Response, AppError> {
    let l = layout(&st, &headers);
    let conf = st.cfg.triage_auto_confidence;
    let paths = st.paths.clone();
    match st.db.read(move |c| load_scenario(c, &paths, id, conf)).await? {
        Some(mut p) => {
            p.l = l;
            p.jira_url = st.cfg.jira_url.clone();
            Ok(render(&p))
        }
        None => Ok(message(
            l,
            StatusCode::NOT_FOUND,
            "Senaryo bulunamadı",
            "Bu numarada bir senaryo yok.",
        )),
    }
}

// ---------------------------------------------------------------------------------------------
// Canlı ekran (SW-3.4)

#[derive(Template)]
#[template(path = "live.html")]
struct LivePage {
    l: Layout,
    run: RunRow,
}

pub async fn live_page(State(st): State<AppState>, headers: HeaderMap, Path(id): Path<i64>) -> Result<Response, AppError> {
    let l = layout(&st, &headers);
    match st.db.read(move |c| queries::run(c, id)).await? {
        Some(run) => Ok(render(&LivePage { l, run })),
        None => Ok(message(l, StatusCode::NOT_FOUND, "Koşum bulunamadı", "Bu numarada bir koşum yok.")),
    }
}

// ---------------------------------------------------------------------------------------------
// Senaryo geçmişi (SW-4.2)

#[derive(Deserialize)]
pub struct HistoryQuery {
    project: i64,
    branch: Option<String>,
    key: String,
    page: Option<i64>,
}

#[derive(Template)]
#[template(path = "history.html")]
struct HistoryPage {
    l: Layout,
    name: String,
    project_path: String,
    branch: Option<String>,
    flaky: bool,
    rows: Vec<ScenarioRow>,
    page: i64,
    qs: String,
}

pub async fn history_page(State(st): State<AppState>, headers: HeaderMap, Query(q): Query<HistoryQuery>) -> Result<Response, AppError> {
    let l = layout(&st, &headers);
    let page = q.page.unwrap_or(1).max(1);
    let branch = q.branch.filter(|b| !b.is_empty());
    let window = st.cfg.flaky_window;
    let (b2, key) = (branch.clone(), q.key.clone());
    let (rows, path, flaky) = st
        .db
        .read(move |c| {
            let rows = crate::history::scenario_history(c, q.project, b2.as_deref(), &key, 50, (page - 1) * 50)?;
            let path: String = c
                .query_row("SELECT path FROM projects WHERE id = ?1", [q.project], |r| r.get(0))
                .unwrap_or_default();
            let flaky = match &b2 {
                Some(b) => crate::history::is_flaky(c, q.project, b, &key, window)?,
                None => false,
            };
            Ok((rows, path, flaky))
        })
        .await?;
    let name = rows.first().map(|r| r.name.clone()).unwrap_or_else(|| "Senaryo".into());
    let qs = format!(
        "project={}&branch={}&key={}",
        q.project,
        urlencoding::encode(branch.as_deref().unwrap_or("")),
        urlencoding::encode(&q.key)
    );
    Ok(render(&HistoryPage {
        l,
        name,
        project_path: path,
        branch,
        flaky,
        rows,
        page,
        qs,
    }))
}

// ---------------------------------------------------------------------------------------------
// Arama ve CSV (SW-4.7, 4.8)

#[derive(Template)]
#[template(path = "search.html")]
struct SearchPage {
    l: Layout,
    f: SearchFilter,
    projects: Vec<Project>,
    categories: &'static [(&'static str, &'static str)],
    statuses: &'static [(&'static str, &'static str)],
    fields: &'static [(&'static str, &'static str)],
    hidden: Vec<(String, String)>,
    rows: Vec<SearchRow>,
    total: i64,
    page: i64,
    pages: i64,
    qs: String,
}

impl SearchPage {
    fn project_selected(&self, id: &i64) -> bool {
        self.f.project == Some(*id)
    }
}

const STATUSES: &[(&str, &str)] = &[
    ("kaldi", "Kaldı (failed + broken)"),
    ("passed", "Geçti"),
    ("failed", "Failed"),
    ("broken", "Broken"),
    ("skipped", "Atlandı"),
    ("unknown", "Bilinmiyor"),
];

/// Sorgu dizesini (boş alanlar ve `?run=` gibi sayılar dahil) SearchFilter'a çevirir.
pub fn parse_filter(raw: Option<&str>) -> (SearchFilter, Vec<(String, String)>) {
    let pairs: Vec<(String, String)> = raw
        .unwrap_or("")
        .split('&')
        .filter_map(|kv| kv.split_once('=').or(Some((kv, ""))))
        .filter(|(k, _)| !k.is_empty())
        .map(|(k, v)| {
            let dec = |s: &str| {
                urlencoding::decode(&s.replace('+', " "))
                    .map(|c| c.into_owned())
                    .unwrap_or_default()
            };
            (dec(k), dec(v))
        })
        .collect();
    let get = |k: &str| {
        pairs
            .iter()
            .find(|(a, _)| a == k)
            .map(|(_, v)| v.clone())
            .filter(|v| !v.trim().is_empty())
    };
    let f = SearchFilter {
        project: get("project").and_then(|v| v.parse().ok()),
        branch: get("branch"),
        run: get("run").and_then(|v| v.trim().trim_start_matches('#').parse().ok()),
        tag: get("tag"),
        feature: get("feature"),
        doors: get("doors"),
        jira: get("jira"),
        triage: get("triage"),
        status: get("status"),
        q: get("q"),
        page: get("page").and_then(|v| v.parse().ok()),
    }
    .normalized();
    (f, pairs)
}

pub async fn search_page(State(st): State<AppState>, headers: HeaderMap, RawQuery(raw): RawQuery) -> Result<Response, AppError> {
    let l = layout(&st, &headers);
    let (f, _) = parse_filter(raw.as_deref());
    let f2 = f.clone();
    let (projects, (rows, total)) = st.db.read(move |c| Ok((queries::projects(c)?, queries::search(c, &f2)?))).await?;
    let page = f.page.unwrap_or(1).max(1);
    let pages = ((total + queries::PAGE_SIZE - 1) / queries::PAGE_SIZE).max(1);
    let qs = f.query_string();
    let hidden = qs
        .split('&')
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.to_string(), urlencoding::decode(v).map(|c| c.into_owned()).unwrap_or_default()))
        .collect();
    Ok(render(&SearchPage {
        l,
        f,
        projects,
        categories: triage::CATEGORIES,
        statuses: STATUSES,
        fields: crate::export::FIELDS,
        hidden,
        rows,
        total,
        page,
        pages,
        qs,
    }))
}

pub async fn export_csv(State(st): State<AppState>, RawQuery(raw): RawQuery) -> Response {
    let (f, pairs) = parse_filter(raw.as_deref());
    // fields=a,b,c (sıralı) ya da formdan alan=a&alan=b
    let fields_param = pairs
        .iter()
        .find(|(k, _)| k == "fields")
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| {
            pairs
                .iter()
                .filter(|(k, _)| k == "alan")
                .map(|(_, v)| v.as_str())
                .collect::<Vec<_>>()
                .join(",")
        });
    let fields = crate::export::parse_fields(Some(&fields_param));
    let sep = if pairs.iter().any(|(k, v)| k == "sep" && v == ";") {
        ';'
    } else {
        ','
    };
    let name = match f.run {
        Some(r) => format!("raporcu-kosum-{r}.csv"),
        None => format!("raporcu-senaryolar-{}.csv", chrono::Local::now().format("%Y%m%d-%H%M")),
    };
    crate::export::stream_csv(&st, f, fields, sep, name)
}

// ---------------------------------------------------------------------------------------------
// DOORS (SW-6.4)

pub async fn doors_csv(State(st): State<AppState>, Path(id): Path<i64>) -> Result<Response, AppError> {
    let rows = st.db.read(move |c| crate::export::doors_rows(c, id)).await?;
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"doors-kosum-{id}.csv\""),
            ),
        ],
        crate::export::doors_csv(&rows, id),
    )
        .into_response())
}

pub async fn doors_dxl(State(st): State<AppState>, Path(id): Path<i64>) -> Result<Response, AppError> {
    let rows = st.db.read(move |c| crate::export::doors_rows(c, id)).await?;
    let url = format!("{}/runs/{id}", st.cfg.public_url);
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "text/plain; charset=utf-8".to_string()),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"doors-kosum-{id}.dxl\""),
            ),
        ],
        crate::export::doors_dxl(&rows, id, &url),
    )
        .into_response())
}

// ---------------------------------------------------------------------------------------------
// Paylaşım (SW-5.1, 5.2)

fn sanitize_run(st: &AppState, run: &mut RunRow, rows: &mut [RunScenario]) {
    let secrets = st.cfg.secrets();
    run.pipeline_url = None;
    run.job_url = None;
    run.project_url = None;
    for r in rows.iter_mut() {
        r.jira.clear();
        r.s.jira_keys.clear();
        r.s.tags.retain(|t| crate::share::visible_tag(t));
        r.s.message = r.s.message.as_deref().map(|m| crate::share::sanitize_text(m, &secrets));
        r.fix = None;
        if let Some(t) = r.triage.as_mut() {
            t.reason = crate::share::sanitize_text(&t.reason, &secrets);
        }
    }
}

async fn share_run_id(st: &AppState, token: &str) -> Result<Option<i64>, AppError> {
    let t = token.to_string();
    Ok(st.db.read(move |c| queries::share_run(c, &t)).await?)
}

fn share_layout(token: &str) -> Layout {
    Layout {
        admin: false,
        banner: None,
        share: Some(token.to_string()),
    }
}

pub async fn share_page(State(st): State<AppState>, Path(token): Path<String>) -> Result<Response, AppError> {
    let Some(id) = share_run_id(&st, &token).await? else {
        return Ok(message(
            share_layout(&token),
            StatusCode::NOT_FOUND,
            "Link geçersiz",
            "Bu paylaşım linki iptal edilmiş ya da hiç yok.",
        ));
    };
    let (conf, window) = (st.cfg.triage_auto_confidence, st.cfg.flaky_window);
    let Some((mut run, mut rows, _)) = st.db.read(move |c| load_run(c, id, conf, window)).await? else {
        return Err(AppError::not_found());
    };
    sanitize_run(&st, &mut run, &mut rows);
    Ok(render(&RunPage::new(share_layout(&token), run, rows, vec![], None)))
}

pub async fn share_scenario(State(st): State<AppState>, Path((token, sid)): Path<(String, i64)>) -> Result<Response, AppError> {
    let Some(run_id) = share_run_id(&st, &token).await? else {
        return Err(AppError::not_found());
    };
    let conf = st.cfg.triage_auto_confidence;
    let paths = st.paths.clone();
    let Some(mut p) = st.db.read(move |c| load_scenario(c, &paths, sid, conf)).await? else {
        return Err(AppError::not_found());
    };
    if p.s.run_id != run_id {
        return Err(AppError::not_found());
    }
    let secrets = st.cfg.secrets();
    p.l = share_layout(&token);
    p.jira.clear();
    p.bugs.clear();
    p.s.jira_keys.clear();
    p.s.tags.retain(|t| crate::share::visible_tag(t));
    p.fixes.clear();
    p.message = p.message.as_deref().map(|m| crate::share::sanitize_text(m, &secrets));
    p.trace = p.trace.as_deref().map(|m| crate::share::sanitize_text(m, &secrets));
    p.attempts.retain(|a| a.run_id == run_id);
    for a in p.attempts.iter_mut() {
        a.message = a.message.as_deref().map(|m| crate::share::sanitize_text(m, &secrets));
    }
    for s in p.steps.iter_mut() {
        s.message = s.message.as_deref().map(|m| crate::share::sanitize_text(m, &secrets));
    }
    p.triage = None;
    p.triage_history.clear();
    Ok(render(&p))
}

pub async fn share_file(State(st): State<AppState>, Path((token, aid)): Path<(String, i64)>, req: Request) -> Result<Response, AppError> {
    let Some(run_id) = share_run_id(&st, &token).await? else {
        return Err(AppError::not_found());
    };
    let Some(att) = st.db.read(move |c| queries::attachment(c, aid)).await? else {
        return Err(AppError::not_found());
    };
    if att.run_id != run_id {
        return Err(AppError::not_found());
    }
    Ok(crate::web::files::serve_attachment_file(&st, &att, req).await)
}

pub async fn create_share(State(st): State<AppState>, _a: Admin, Path(id): Path<i64>) -> Result<Response, AppError> {
    let token = auth::random_token();
    let t2 = token.clone();
    st.db
        .write(move |c| {
            c.execute(
                "INSERT INTO share_links (token, run_id, created_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![t2, id, crate::db::now()],
            )?;
            Ok(())
        })
        .await?;
    tracing::info!(run_id = id, "paylaşım linki oluşturuldu");
    Ok(Redirect::to(&format!("/runs/{id}?notice=share:{token}")).into_response())
}

pub async fn revoke_share(State(st): State<AppState>, _a: Admin, Path(id): Path<i64>) -> Result<Response, AppError> {
    let run_id: Option<i64> = st
        .db
        .write(move |c| {
            c.execute(
                "UPDATE share_links SET revoked_at = ?2 WHERE id = ?1 AND revoked_at IS NULL",
                rusqlite::params![id, crate::db::now()],
            )?;
            Ok(c.query_row("SELECT run_id FROM share_links WHERE id = ?1", [id], |r| r.get(0)).ok())
        })
        .await?;
    tracing::info!(paylasim_id = id, run_id = run_id.unwrap_or(0), "paylaşım linki iptal edildi");
    Ok(Redirect::to("/admin").into_response())
}

// ---------------------------------------------------------------------------------------------
// Triage kararı (SW-4.5)

#[derive(Deserialize)]
pub struct TriageForm {
    category: String,
    reason: String,
}

pub async fn set_triage(
    State(st): State<AppState>,
    _a: Admin,
    Path(id): Path<i64>,
    Form(f): Form<TriageForm>,
) -> Result<Response, AppError> {
    if !triage::is_category(&f.category) {
        return Err(AppError::bad("geçersiz kategori"));
    }
    let conf = st.cfg.triage_auto_confidence;
    st.db
        .write(move |c| {
            triage::add(c, id, &f.category, "admin", f.reason.trim(), None)?;
            triage::refresh(c, id, conf)?;
            Ok(())
        })
        .await?;
    Ok(Redirect::to(&format!("/scenarios/{id}")).into_response())
}

// ---------------------------------------------------------------------------------------------
// Admin paneli (SW-5.2, SW-10.7, SW-10.8, SW-6.3)

#[derive(Template)]
#[template(path = "admin.html")]
struct AdminPage {
    l: Layout,
    shares: Vec<queries::ShareRow>,
    logs: Vec<(String, String, String, String)>,
    jira_actions: Vec<(i64, String, String, String, String)>,
    disk_pct: String,
    jira_mode: &'static str,
    version: &'static str,
    public_url: String,
    notice: Option<String>,
}

pub async fn admin_page(
    State(st): State<AppState>,
    _a: Admin,
    headers: HeaderMap,
    Query(nq): Query<NoticeQuery>,
) -> Result<Response, AppError> {
    let l = layout(&st, &headers);
    let (shares, jira_actions) = st
        .db
        .read(|c| {
            let shares = queries::share_links(c, 200)?;
            let mut s = c.prepare("SELECT run_id, action, COALESCE(jira_key, ''), COALESCE(detail, ''), created_at FROM jira_actions ORDER BY id DESC LIMIT 100")?;
            let ja = s.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?.collect::<rusqlite::Result<_>>()?;
            Ok((shares, ja))
        })
        .await?;
    let logs = st
        .recent_logs
        .list()
        .into_iter()
        .map(|v| {
            let mut rest = v.as_object().cloned().unwrap_or_default();
            let take = |m: &mut serde_json::Map<String, serde_json::Value>, k: &str| {
                m.remove(k).and_then(|x| x.as_str().map(String::from)).unwrap_or_default()
            };
            let (z, s, o) = (take(&mut rest, "zaman"), take(&mut rest, "seviye"), take(&mut rest, "olay"));
            (z, s, o, serde_json::Value::Object(rest).to_string())
        })
        .collect();
    let d = st.disk.get();
    Ok(render(&AdminPage {
        l,
        shares,
        logs,
        jira_actions,
        disk_pct: format!("%{:.0} dolu{}", d.used_pct, if d.warning { " (uyarı)" } else { "" }),
        jira_mode: match st.cfg.jira_mode {
            crate::config::JiraMode::Off => "kapalı",
            crate::config::JiraMode::Observe => "gözlem modu",
            crate::config::JiraMode::On => "açık",
        },
        version: common::VERSION,
        public_url: st.cfg.public_url.clone(),
        notice: nq.notice,
    }))
}

// ---------------------------------------------------------------------------------------------
// Self-healing paneli (SW-7.5)

pub struct Day {
    pub day: String,
    pub attempts: i64,
    pub mrs: i64,
    pub merged: i64,
    pub rejected: i64,
    pub h_merged: i64,
    pub h_rejected: i64,
    pub h_open: i64,
    pub h_other: i64,
}

#[derive(Template)]
#[template(path = "healing.html")]
struct HealingPage {
    l: Layout,
    days: Vec<Day>,
    totals: (i64, i64, i64, i64),
    chart: String,
}

pub fn healing_days(c: &rusqlite::Connection, today: chrono::NaiveDate) -> anyhow::Result<Vec<Day>> {
    let start = today - chrono::Duration::days(29);
    let mut st = c.prepare(
        "SELECT substr(created_at, 1, 10) AS d, COUNT(*),
                SUM(mr_iid IS NOT NULL), SUM(mr_state = 'merged'), SUM(mr_state = 'closed')
         FROM fix_attempts WHERE substr(created_at, 1, 10) >= ?1 GROUP BY d",
    )?;
    let mut map = std::collections::HashMap::new();
    let mut rows = st.query([start.format("%Y-%m-%d").to_string()])?;
    while let Some(r) = rows.next()? {
        map.insert(
            r.get::<_, String>(0)?,
            (
                r.get::<_, i64>(1)?,
                r.get::<_, Option<i64>>(2)?.unwrap_or(0),
                r.get::<_, Option<i64>>(3)?.unwrap_or(0),
                r.get::<_, Option<i64>>(4)?.unwrap_or(0),
            ),
        );
    }
    let max = map.values().map(|v| v.0).max().unwrap_or(1).max(1);
    let scale = |n: i64| n * 110 / max;
    Ok((0..30)
        .map(|i| {
            let d = (start + chrono::Duration::days(i)).format("%Y-%m-%d").to_string();
            let (a, m, mg, rj) = map.get(&d).copied().unwrap_or((0, 0, 0, 0));
            Day {
                h_merged: scale(mg),
                h_rejected: scale(rj),
                h_open: scale((m - mg - rj).max(0)),
                h_other: scale((a - m).max(0)),
                day: d,
                attempts: a,
                mrs: m,
                merged: mg,
                rejected: rj,
            }
        })
        .collect())
}

pub async fn healing_page(State(st): State<AppState>, headers: HeaderMap) -> Result<Response, AppError> {
    let l = layout(&st, &headers);
    let days = st.db.read(|c| healing_days(c, chrono::Utc::now().date_naive())).await?;
    let totals = days.iter().fold((0, 0, 0, 0), |t, d| {
        (t.0 + d.attempts, t.1 + d.mrs, t.2 + d.merged, t.3 + d.rejected)
    });
    use crate::charts::{Column, Slice};
    let cols: Vec<Column> = days
        .iter()
        .map(|d| Column {
            label: format!("{}.{}", d.day.get(8..10).unwrap_or(""), d.day.get(5..7).unwrap_or("")),
            href: None,
            title: d.day.clone(),
            parts: vec![
                Slice {
                    class: "gecti",
                    name: "Birleştirilen",
                    value: d.merged,
                },
                Slice {
                    class: "kaldi",
                    name: "Reddedilen",
                    value: d.rejected,
                },
                Slice {
                    class: "diger",
                    name: "Açık MR",
                    value: (d.mrs - d.merged - d.rejected).max(0),
                },
                Slice {
                    class: "bos",
                    name: "MR'sız deneme",
                    value: (d.attempts - d.mrs).max(0),
                },
            ],
        })
        .collect();
    let chart = crate::charts::stacked_columns(&cols, "Gece bazında OpenCode denemeleri");
    Ok(render(&HealingPage { l, days, totals, chart }))
}

// ---------------------------------------------------------------------------------------------
// Admin işlemleri: Jira'ya bug aç (SW-6.1), OpenCode ile düzelt (SW-7.4)

pub async fn create_jira(State(st): State<AppState>, _a: Admin, headers: HeaderMap, Path(id): Path<i64>) -> Result<Response, AppError> {
    let l = layout(&st, &headers);
    let Some(j) = crate::jira::Jira::from_config(&st.cfg) else {
        return Ok(message(
            l,
            StatusCode::BAD_REQUEST,
            "Jira ayarlı değil",
            "JIRA_URL, JIRA_TOKEN ve JIRA_PROJECT ayarlanmalı.",
        ));
    };
    let (paths, cfg) = (st.paths.clone(), st.cfg.clone());
    match st
        .db
        .write(move |c| crate::jira::create_for_scenario(c, &paths, &cfg, &j, id))
        .await
    {
        Ok(_) => Ok(Redirect::to(&format!("/scenarios/{id}")).into_response()),
        Err(e) => Ok(message(l, StatusCode::BAD_GATEWAY, "Jira bug açılamadı", &e.to_string())),
    }
}

pub async fn trigger_fix(State(st): State<AppState>, _a: Admin, headers: HeaderMap, Path(id): Path<i64>) -> Result<Response, AppError> {
    let l = layout(&st, &headers);
    let info: Option<(Option<String>, String, String)> = st
        .db
        .read(move |c| {
            Ok(c.query_row(
                "SELECT p.gitlab_id, p.path, r.branch FROM runs r JOIN projects p ON p.id = r.project_id WHERE r.id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .ok())
        })
        .await?;
    let Some((gid, path, branch)) = info else {
        return Err(AppError::not_found());
    };
    let project = gid.unwrap_or(path);
    let st2 = st.clone();
    let res = tokio::task::spawn_blocking(move || crate::gitlab::trigger_fix_pipeline(&st2, &project, &branch, id)).await;
    match res {
        Ok(Ok(v)) => {
            tracing::info!(
                run_id = id,
                pipeline_id = v["id"].as_i64().unwrap_or(0),
                "OpenCode fix işi tetiklendi"
            );
            Ok(Redirect::to(&format!("/runs/{id}?notice=fix")).into_response())
        }
        Ok(Err(e)) => Ok(message(l, StatusCode::BAD_GATEWAY, "Fix işi tetiklenemedi", &e.to_string())),
        Err(e) => Err(anyhow::Error::from(e).into()),
    }
}
