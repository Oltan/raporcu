//! SW-4.2, SW-4.6: senaryo geçmişi ve flaky tespiti — her zaman proje (ve isteğe bağlı branch) bazında.

use crate::queries::{scenario_row, ScenarioRow, SCEN_COLS};
use anyhow::Result;
use rusqlite::{params, Connection};
use std::collections::HashSet;

/// Senaryonun geçmiş koşumlardaki son denemeleri, en yeniden eskiye.
pub fn scenario_history(c: &Connection, project_id: i64, branch: Option<&str>, key: &str, limit: i64, offset: i64) -> Result<Vec<ScenarioRow>> {
    let mut st = c.prepare(&format!(
        "SELECT {SCEN_COLS} FROM scenarios s
         WHERE s.project_id = ?1 AND (?2 IS NULL OR s.branch = ?2) AND s.scenario_key = ?3 AND s.is_latest = 1
         ORDER BY s.run_id DESC LIMIT ?4 OFFSET ?5"
    ))?;
    let rows = st.query_map(params![project_id, branch, key, limit, offset], scenario_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Flaky senaryolar: son `window` koşumda hem geçmiş hem kalmış olanlar ya da aynı koşumda önce
/// kalıp tekrar denemede geçenler (SW-4.9).
pub fn flaky_keys(c: &Connection, project_id: i64, branch: &str, window: usize) -> Result<HashSet<String>> {
    let mut st = c.prepare(
        "WITH recent AS (SELECT id FROM runs WHERE project_id = ?1 AND branch = ?2 ORDER BY id DESC LIMIT ?3)
         SELECT scenario_key FROM scenarios
          WHERE run_id IN (SELECT id FROM recent)
          GROUP BY scenario_key
         HAVING SUM(CASE WHEN status = 'passed' AND is_latest = 1 THEN 1 ELSE 0 END) > 0
            AND SUM(CASE WHEN status IN ('failed', 'broken') THEN 1 ELSE 0 END) > 0",
    )?;
    let rows = st.query_map(params![project_id, branch, window as i64], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn is_flaky(c: &Connection, project_id: i64, branch: &str, key: &str, window: usize) -> Result<bool> {
    Ok(flaky_keys(c, project_id, branch, window)?.contains(key))
}
