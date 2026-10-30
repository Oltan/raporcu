//! SW-7.1 fix-candidates sınıflandırması, SW-7.3 fix-result kaydı, SW-7.7 geçmiş düzeltme örnekleri.

use crate::db::now;
use anyhow::Result;
use common::api::{FixCandidate, FixExample, FixResult};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;

/// Hedef: `opencode` | `jira` | `atla` | `bildirim`
pub fn target(status: &str, tags: &[String], category: Option<&str>) -> &'static str {
    if tags.iter().any(|t| t == "sample-fail") {
        return "atla";
    }
    match category {
        Some("test_kodu") => "opencode",
        Some("urun") => "jira",
        Some("flaky") => "atla",
        Some("ortam") => "bildirim",
        _ => match status {
            "broken" => "opencode",
            "failed" => "jira",
            _ => "atla",
        },
    }
}

fn words(s: &str) -> HashSet<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 3 && !w.chars().all(|c| c.is_ascii_digit()))
        .map(|w| w.to_lowercase())
        .collect()
}

fn similarity(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    a.intersection(b).count() as f64 / a.union(b).count() as f64
}

/// SW-7.7: aynı projede birleştirilmiş düzeltmelerden en benzer olanlar.
pub fn examples(c: &Connection, project_id: i64, name: &str, message: &str, limit: usize) -> Result<Vec<FixExample>> {
    let mut st = c.prepare(
        "SELECT COALESCE(s.name, f.scenario_key), s.message, f.diff FROM fix_attempts f LEFT JOIN scenarios s ON s.id = f.scenario_id
         WHERE f.project_id = ?1 AND f.mr_state = 'merged' AND f.diff IS NOT NULL AND f.diff <> '' ORDER BY f.id DESC LIMIT 200",
    )?;
    let me = words(&format!("{name} {message}"));
    let mut scored: Vec<(f64, FixExample)> = st
        .query_map([project_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?, r.get::<_, String>(2)?))
        })?
        .filter_map(|x| x.ok())
        .map(|(n, m, d)| {
            let score = similarity(&me, &words(&format!("{n} {}", m.as_deref().unwrap_or(""))));
            (
                score,
                FixExample {
                    scenario: n,
                    message: m.map(|m| crate::logging::truncate(&m, 500)),
                    diff: crate::logging::truncate(&d, 8000),
                },
            )
        })
        .filter(|(s, _)| *s > 0.1)
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    Ok(scored.into_iter().take(limit).map(|(_, e)| e).collect())
}

pub fn candidates(c: &Connection, paths: &crate::Paths, run_id: i64, public_url: &str) -> Result<Vec<FixCandidate>> {
    let project_id: i64 = c.query_row("SELECT project_id FROM runs WHERE id = ?1", [run_id], |r| r.get(0))?;
    let mut st = c.prepare(
        "SELECT id, scenario_key, name, feature, feature_file, status, message, trace, tags, triage_category
         FROM scenarios WHERE run_id = ?1 AND is_latest = 1 AND status IN ('failed', 'broken') ORDER BY id",
    )?;
    let rows: Vec<(
        i64,
        String,
        String,
        Option<String>,
        Option<String>,
        String,
        Option<String>,
        Option<String>,
        String,
        Option<String>,
    )> = st
        .query_map([run_id], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
                r.get(8)?,
                r.get(9)?,
            ))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut out = Vec::new();
    for (id, key, name, feature, ffile, status, msg, trace, tags, cat) in rows {
        let tags: Vec<String> = tags.split_whitespace().map(String::from).collect();
        let t = target(&status, &tags, cat.as_deref());
        let steps = crate::queries::steps(c, id)?
            .into_iter()
            .map(|s| format!("{} [{}]", s.name, s.status))
            .collect();
        let shot = crate::queries::first_screenshot(c, id)?.map(|a| format!("{public_url}/files/{}", a.id));
        let ex = if t == "opencode" {
            examples(c, project_id, &name, msg.as_deref().unwrap_or(""), 2)?
        } else {
            vec![]
        };
        // küçük metin ekleri (sahne ağacı, log): ilk 3 tanesi, her biri en fazla 16 KB
        let texts = crate::queries::attachments(c, id)?
            .into_iter()
            .filter(|a| a.is_text() && !a.is_deleted())
            .filter_map(|a| {
                let data = std::fs::read(paths.attachment(a.run_id, &a.source)).ok()?;
                let text = String::from_utf8_lossy(&data[..data.len().min(16 * 1024)]).into_owned();
                Some(common::api::FixText {
                    name: a.title(),
                    content: text,
                })
            })
            .filter(|t| !t.content.trim().is_empty())
            .take(3)
            .collect();
        out.push(FixCandidate {
            scenario_id: id,
            scenario_key: key,
            name,
            feature,
            feature_file: ffile,
            status,
            target: t.to_string(),
            message: msg,
            trace: trace.map(|t| crate::logging::truncate(&t, 6000)),
            steps,
            screenshot_url: shot,
            tags,
            examples: ex,
            texts,
        });
    }
    Ok(out)
}

/// SW-7.3: fix sonucu kaydedilir; OpenCode triage önerisi karar olarak eklenir.
pub fn record(c: &Connection, run_id: i64, r: &FixResult, auto_conf: f64) -> Result<i64> {
    let (project_id,): (i64,) = c.query_row("SELECT project_id FROM runs WHERE id = ?1", [run_id], |x| Ok((x.get(0)?,)))?;
    let sid: Option<i64> = c
        .query_row(
            "SELECT id FROM scenarios WHERE run_id = ?1 AND scenario_key = ?2 AND is_latest = 1",
            params![run_id, r.scenario_key],
            |x| x.get(0),
        )
        .optional()?;
    let mr_state = r.mr_iid.map(|_| "opened");
    c.execute(
        "INSERT INTO fix_attempts (run_id, scenario_id, project_id, scenario_key, outcome, mr_iid, mr_url, mr_state, branch,
                                   validation_passed, summary, warnings, diff, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)",
        params![
            run_id,
            sid,
            project_id,
            r.scenario_key,
            r.outcome,
            r.mr_iid,
            r.mr_url,
            mr_state,
            r.branch,
            r.validation_passed.map(|v| v as i64),
            r.summary.as_deref().map(|s| crate::logging::truncate(s, 2000)),
            serde_json::to_string(&r.warnings)?,
            r.diff.as_deref().map(|d| crate::logging::truncate(d, 200_000)),
            now()
        ],
    )?;
    let id = c.last_insert_rowid();
    tracing::info!(run_id, senaryo_id = sid.unwrap_or(0), sonuc = %r.outcome, mr = r.mr_iid.unwrap_or(0), "OpenCode denemesi bitti");
    if let (Some(sid), Some(t)) = (sid, &r.triage) {
        if crate::triage::is_category(&t.category) {
            crate::triage::add(c, sid, &t.category, "opencode", &t.reason, Some(t.confidence.clamp(0.0, 1.0)))?;
            crate::triage::refresh(c, sid, auto_conf)?;
        }
    }
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_7_1_target_for_each_case() {
        let none: Vec<String> = vec![];
        assert_eq!(target("broken", &none, Some("test_kodu")), "opencode");
        assert_eq!(target("broken", &none, Some("urun")), "jira");
        assert_eq!(target("failed", &none, Some("flaky")), "atla");
        assert_eq!(target("failed", &none, Some("ortam")), "bildirim");
        assert_eq!(target("broken", &none, None), "opencode");
        assert_eq!(target("failed", &none, None), "jira");
        assert_eq!(target("broken", &none, Some("bilinmiyor")), "opencode");
        assert_eq!(target("broken", &["sample-fail".to_string()], Some("test_kodu")), "atla");
    }
}
