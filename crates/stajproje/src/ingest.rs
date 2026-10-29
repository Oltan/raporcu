//! Allure sonuçlarının veritabanına işlenmesi (SW-2.1, 2.2, 3.3, 4.6, 4.9).
//!
//! Tüm fonksiyonlar bir transaction içinde çağrılır. Aynı result `uuid`'si bir koşumda bir kez
//! sayılır; aynı `scenario_key` (historyId) birden çok kez gelirse en geç başlayan deneme
//! senaryonun sonucudur, öncekiler deneme listesidir.

use crate::db::now;
use anyhow::{Context, Result};
use common::allure::{Container, Step, TestResult};
use common::api::CiInfo;
use rusqlite::{params, Connection, OptionalExtension};

pub const MAX_TRACE_CHARS: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub struct RunCtx {
    pub run_id: i64,
    pub project_id: i64,
    pub branch: String,
}

/// SW-4.6: proje GitLab bilgisinden kendiliğinden oluşur. Kimlik önce GitLab proje id'si, yoksa yol.
pub fn ensure_project(c: &Connection, ci: &CiInfo) -> Result<i64> {
    anyhow::ensure!(!ci.project_path.trim().is_empty(), "project_path boş olamaz");
    if let Some(gid) = ci.project_id.as_deref().filter(|s| !s.is_empty()) {
        if let Some(id) = c
            .query_row("SELECT id FROM projects WHERE gitlab_id = ?1", [gid], |r| r.get::<_, i64>(0))
            .optional()?
        {
            c.execute(
                "UPDATE projects SET path = ?2, url = COALESCE(?3, url) WHERE id = ?1 AND path <> ?2",
                params![id, ci.project_path, ci.project_url],
            )?;
            return Ok(id);
        }
    }
    if let Some(id) = c
        .query_row("SELECT id FROM projects WHERE path = ?1", [&ci.project_path], |r| {
            r.get::<_, i64>(0)
        })
        .optional()?
    {
        c.execute(
            "UPDATE projects SET gitlab_id = COALESCE(gitlab_id, ?2), url = COALESCE(url, ?3) WHERE id = ?1",
            params![id, ci.project_id, ci.project_url],
        )?;
        return Ok(id);
    }
    c.execute(
        "INSERT INTO projects (gitlab_id, path, url, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![ci.project_id, ci.project_path, ci.project_url, now()],
    )?;
    let id = c.last_insert_rowid();
    tracing::info!(proje = %ci.project_path, proje_id = id, "proje oluşturuldu");
    Ok(id)
}

pub fn create_run(c: &Connection, project_id: i64, ci: &CiInfo, status: &str, total_expected: Option<i64>) -> Result<RunCtx> {
    let branch = if ci.branch.trim().is_empty() {
        "unknown".to_string()
    } else {
        ci.branch.clone()
    };
    let t = now();
    c.execute(
        "INSERT INTO runs (project_id, branch, commit_sha, pipeline_id, pipeline_url, job_id, job_url, os, status,
                           total_expected, started_at, last_activity_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
        params![
            project_id,
            branch,
            ci.commit,
            ci.pipeline_id,
            ci.pipeline_url,
            ci.job_id,
            ci.job_url,
            ci.os,
            status,
            total_expected,
            t
        ],
    )?;
    let run_id = c.last_insert_rowid();
    tracing::info!(run_id, proje = %ci.project_path, branch = %branch, pipeline_id = ci.pipeline_id.as_deref().unwrap_or(""),
        beklenen = total_expected.unwrap_or(-1), "koşum açıldı");
    Ok(RunCtx {
        run_id,
        project_id,
        branch,
    })
}

pub fn run_ctx(c: &Connection, run_id: i64) -> Result<Option<RunCtx>> {
    Ok(c.query_row("SELECT project_id, branch FROM runs WHERE id = ?1", [run_id], |r| {
        Ok(RunCtx {
            run_id,
            project_id: r.get(0)?,
            branch: r.get(1)?,
        })
    })
    .optional()?)
}

fn space_list(items: &[String]) -> String {
    if items.is_empty() {
        String::new()
    } else {
        format!(" {} ", items.join(" "))
    }
}

/// Sonucu kaydeder. Aynı uuid daha önce geldiyse `None` döner (SW-3.3).
pub fn ingest_result(c: &Connection, run: &RunCtx, r: &TestResult) -> Result<Option<i64>> {
    anyhow::ensure!(!r.uuid.is_empty(), "result uuid boş");
    let exists: bool = c.query_row(
        "SELECT EXISTS(SELECT 1 FROM scenarios WHERE run_id = ?1 AND result_uuid = ?2)",
        params![run.run_id, r.uuid],
        |x| x.get(0),
    )?;
    if exists {
        return Ok(None);
    }
    let feature_file = r.label("gherkin_uri").map(|u| u.trim_start_matches("classpath:").to_string());
    let params_json = serde_json::to_string(&r.parameters)?;
    let trace = r.trace().map(|t| crate::logging::truncate(t, MAX_TRACE_CHARS));
    c.execute(
        "INSERT INTO scenarios (run_id, project_id, branch, result_uuid, scenario_key, name, full_name, feature, feature_file,
                                tags, doors_ids, jira_keys, status, message, trace, start_ms, stop_ms, duration_ms, parameters, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
        params![
            run.run_id,
            run.project_id,
            run.branch,
            r.uuid,
            r.scenario_key(),
            r.name,
            r.full_name,
            r.feature(),
            feature_file,
            space_list(&r.tags()),
            space_list(&r.doors_ids()),
            space_list(&r.jira_keys()),
            r.status().as_str(),
            r.message(),
            trace,
            r.start,
            r.stop,
            r.duration_ms(),
            params_json,
            now()
        ],
    )?;
    let sid = c.last_insert_rowid();
    for a in &r.attachments {
        insert_attachment(c, run.run_id, Some(sid), None, None, "result", a)?;
    }
    let mut pos = 0;
    insert_steps(c, run.run_id, sid, None, 0, &r.steps, &mut pos)?;
    Ok(Some(sid))
}

fn insert_steps(c: &Connection, run_id: i64, sid: i64, parent: Option<i64>, depth: i64, steps: &[Step], pos: &mut i64) -> Result<()> {
    for s in steps {
        *pos += 1;
        let msg = s.status_details.as_ref().and_then(|d| d.message.as_deref());
        c.execute(
            "INSERT INTO steps (scenario_id, parent_id, position, depth, name, status, message, start_ms, stop_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                sid,
                parent,
                *pos,
                depth,
                s.name,
                s.status.unwrap_or_default().as_str(),
                msg,
                s.start,
                s.stop
            ],
        )?;
        let step_id = c.last_insert_rowid();
        for a in &s.attachments {
            insert_attachment(c, run_id, Some(sid), Some(step_id), None, "step", a)?;
        }
        insert_steps(c, run_id, sid, Some(step_id), depth + 1, &s.steps, pos)?;
    }
    Ok(())
}

fn insert_attachment(
    c: &Connection,
    run_id: i64,
    sid: Option<i64>,
    step_id: Option<i64>,
    container_id: Option<i64>,
    origin: &str,
    a: &common::allure::Attachment,
) -> Result<()> {
    if !common::allure::is_safe_file_name(&a.source) {
        tracing::warn!(run_id, kaynak = %a.source, "güvensiz ek adı atlandı");
        return Ok(());
    }
    c.execute(
        "INSERT OR IGNORE INTO attachments (run_id, scenario_id, step_id, container_id, origin, name, mime, source, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![run_id, sid, step_id, container_id, origin, a.name, a.kind, a.source, now()],
    )?;
    Ok(())
}

/// SW-2.2: container dosyası. `@Before/@After` ekleri `children` üzerinden senaryoya bağlanır.
pub fn ingest_container(c: &Connection, run_id: i64, ct: &Container) -> Result<bool> {
    anyhow::ensure!(!ct.uuid.is_empty(), "container uuid boş");
    let inserted = c.execute(
        "INSERT OR IGNORE INTO containers (run_id, uuid) VALUES (?1, ?2)",
        params![run_id, ct.uuid],
    )?;
    if inserted == 0 {
        return Ok(false);
    }
    let cid = c.last_insert_rowid();
    for ch in &ct.children {
        c.execute(
            "INSERT OR IGNORE INTO container_children (container_id, result_uuid) VALUES (?1, ?2)",
            params![cid, ch],
        )?;
    }
    for (origin, fixtures) in [("before", &ct.befores), ("after", &ct.afters)] {
        for f in fixtures.iter() {
            for a in &f.attachments {
                insert_attachment(c, run_id, None, None, Some(cid), origin, a)?;
            }
            fn walk<'a>(steps: &'a [Step], out: &mut Vec<&'a common::allure::Attachment>) {
                for s in steps {
                    out.extend(s.attachments.iter());
                    walk(&s.steps, out);
                }
            }
            let mut nested = Vec::new();
            walk(&f.steps, &mut nested);
            for a in nested {
                insert_attachment(c, run_id, None, None, Some(cid), origin, a)?;
            }
        }
    }
    Ok(true)
}

/// Container ekleri, sonuç dosyası hangi sırayla gelirse gelsin senaryoya bağlanır.
pub fn link_fixture_attachments(c: &Connection, run_id: i64) -> Result<usize> {
    Ok(c.execute(
        "UPDATE attachments SET scenario_id = (
             SELECT s.id FROM container_children cc
             JOIN scenarios s ON s.run_id = attachments.run_id AND s.result_uuid = cc.result_uuid
             WHERE cc.container_id = attachments.container_id
             ORDER BY s.id LIMIT 1)
         WHERE run_id = ?1 AND scenario_id IS NULL AND container_id IS NOT NULL",
        [run_id],
    )?)
}

/// SW-4.9: deneme numaraları, son deneme işareti ve koşum sayaçları.
pub fn recompute_run(c: &Connection, run_id: i64) -> Result<()> {
    c.execute(
        "WITH ranked AS (
             SELECT id,
                    ROW_NUMBER() OVER (PARTITION BY scenario_key ORDER BY COALESCE(start_ms, 0), id) AS rn,
                    COUNT(*) OVER (PARTITION BY scenario_key) AS cnt
             FROM scenarios WHERE run_id = ?1)
         UPDATE scenarios
            SET attempt = (SELECT rn FROM ranked WHERE ranked.id = scenarios.id),
                is_latest = (SELECT rn = cnt FROM ranked WHERE ranked.id = scenarios.id)
          WHERE run_id = ?1",
        [run_id],
    )?;
    c.execute(
        "UPDATE runs SET
            total   = (SELECT COUNT(*) FROM scenarios WHERE run_id = ?1 AND is_latest = 1),
            passed  = (SELECT COUNT(*) FROM scenarios WHERE run_id = ?1 AND is_latest = 1 AND status = 'passed'),
            failed  = (SELECT COUNT(*) FROM scenarios WHERE run_id = ?1 AND is_latest = 1 AND status = 'failed'),
            broken  = (SELECT COUNT(*) FROM scenarios WHERE run_id = ?1 AND is_latest = 1 AND status = 'broken'),
            skipped = (SELECT COUNT(*) FROM scenarios WHERE run_id = ?1 AND is_latest = 1 AND status = 'skipped'),
            unknown = (SELECT COUNT(*) FROM scenarios WHERE run_id = ?1 AND is_latest = 1 AND status = 'unknown'),
            last_activity_at = ?2
         WHERE id = ?1",
        params![run_id, now()],
    )?;
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RunCounts {
    pub status: String,
    pub total_expected: Option<i64>,
    pub total: i64,
    pub passed: i64,
    pub failed: i64,
    pub broken: i64,
    pub skipped: i64,
    pub unknown: i64,
}

pub fn run_counts(c: &Connection, run_id: i64) -> Result<RunCounts> {
    c.query_row(
        "SELECT status, total_expected, total, passed, failed, broken, skipped, unknown FROM runs WHERE id = ?1",
        [run_id],
        |r| {
            Ok(RunCounts {
                status: r.get(0)?,
                total_expected: r.get(1)?,
                total: r.get(2)?,
                passed: r.get(3)?,
                failed: r.get(4)?,
                broken: r.get(5)?,
                skipped: r.get(6)?,
                unknown: r.get(7)?,
            })
        },
    )
    .context("koşum yok")
}

/// Koşumu bitirir (finished / interrupted).
pub fn finish_run(c: &Connection, run_id: i64, status: &str) -> Result<RunCounts> {
    c.execute(
        "UPDATE runs SET status = ?2, finished_at = COALESCE(finished_at, ?3), last_activity_at = ?3 WHERE id = ?1",
        params![run_id, status, now()],
    )?;
    let k = run_counts(c, run_id)?;
    if status == "interrupted" {
        tracing::warn!(
            run_id,
            toplam = k.total,
            gecen = k.passed,
            kalan = k.failed + k.broken,
            "koşum interrupted oldu"
        );
    } else {
        tracing::info!(
            run_id,
            toplam = k.total,
            gecen = k.passed,
            kalan = k.failed + k.broken,
            "koşum bitti"
        );
    }
    Ok(k)
}
