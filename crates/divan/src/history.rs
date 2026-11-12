//! SW-4.2, SW-4.6: senaryo geçmişi ve flaky tespiti — her zaman proje (ve isteğe bağlı branch) bazında.

use crate::queries::{scenario_row, ScenarioRow, SCEN_COLS};
use anyhow::Result;
use rusqlite::{params, Connection};
use std::collections::HashSet;

/// Senaryonun geçmiş koşumlardaki son denemeleri, en yeniden eskiye.
pub fn scenario_history(
    c: &Connection,
    project_id: i64,
    branch: Option<&str>,
    key: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<ScenarioRow>> {
    let mut st = c.prepare(&format!(
        "SELECT {SCEN_COLS} FROM scenarios s
         WHERE s.project_id = ?1 AND (?2 IS NULL OR s.branch = ?2) AND s.scenario_key = ?3 AND s.is_latest = 1
         ORDER BY s.run_id DESC LIMIT ?4 OFFSET ?5"
    ))?;
    let rows = st.query_map(params![project_id, branch, key, limit, offset], scenario_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Flaky senaryolar: son `window` koşumda **aynı commit'te kaldıktan sonra geçenler** (sonraki bir
/// koşumda ya da aynı koşumun tekrar denemesinde, SW-4.9). Kod değişmeden sonuç değiştiyse flaky'dir.
/// Geçerken kalmaya başlayan ve o günden beri geçmeyen senaryo bozulmadır, flaky sayılmaz; commit
/// değişince geçen senaryo da düzeltilmiş sayılır. Commit bilinmiyorsa koşumlar aynı sürüm kabul edilir.
pub fn flaky_keys(c: &Connection, project_id: i64, branch: &str, window: usize) -> Result<HashSet<String>> {
    let mut st = c.prepare(
        "WITH recent AS (SELECT id, COALESCE(commit_sha, '') AS surum FROM runs
                          WHERE project_id = ?1 AND branch = ?2 ORDER BY id DESC LIMIT ?3)
         SELECT DISTINCT f.scenario_key
           FROM scenarios f
           JOIN recent rf ON rf.id = f.run_id
           JOIN recent rp ON rp.surum = rf.surum AND rp.id >= rf.id
           JOIN scenarios p ON p.run_id = rp.id AND p.scenario_key = f.scenario_key AND p.status = 'passed'
          WHERE f.status IN ('failed', 'broken')
            AND (p.run_id > f.run_id OR p.attempt > f.attempt)",
    )?;
    let rows = st.query_map(params![project_id, branch, window as i64], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn is_flaky(c: &Connection, project_id: i64, branch: &str, key: &str, window: usize) -> Result<bool> {
    Ok(flaky_keys(c, project_id, branch, window)?.contains(key))
}
