//! SW-10.8: disk doluluğu ve temizlik.
//!
//! - %80 (DISK_WARN_PCT): admin panelinde ve logda uyarı.
//! - %90 (DISK_CLEAN_PCT): en eski koşumlardan başlayarak önce videolar, sonra ekran görüntüleri
//!   silinir; doluluk uyarı eşiğinin altına inince durur.
//! - Jira issue'su olan senaryonun eki yerine "Silindi, bkz. JIRA-123" yazılır.
//! - Koşum, senaryo, adım, triage kayıtları hiçbir zaman silinmez.

use crate::paths::Paths;
use anyhow::Result;
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};

pub trait DiskProbe: Send + Sync {
    /// Kullanım yüzdesi (0-100)
    fn used_pct(&self) -> Result<f64>;
}

pub struct FsProbe(pub std::path::PathBuf);

impl DiskProbe for FsProbe {
    fn used_pct(&self) -> Result<f64> {
        let total = fs4::total_space(&self.0)? as f64;
        let avail = fs4::available_space(&self.0)? as f64;
        if total <= 0.0 {
            return Ok(0.0);
        }
        Ok((total - avail) / total * 100.0)
    }
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct DiskStatus {
    pub used_pct: f64,
    pub warning: bool,
    pub last_cleanup: Option<String>,
}

#[derive(Clone, Default)]
pub struct DiskState(pub Arc<Mutex<DiskStatus>>);

impl DiskState {
    pub fn get(&self) -> DiskStatus {
        self.0.lock().unwrap().clone()
    }
    pub fn warning_text(&self) -> Option<String> {
        let s = self.get();
        s.warning.then(|| format!("Disk %{:.0} dolu. %{:.0} üstünde eski videolar ve ekran görüntüleri silinir.", s.used_pct, 90.0))
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct CleanupReport {
    pub deleted_files: usize,
    pub freed_bytes: u64,
}

/// Bir ekin silinmesi: dosya gider, kayıt kalır, nedeni yazılır.
fn delete_attachment(c: &Connection, paths: &Paths, id: i64, run_id: i64, source: &str, scenario_id: Option<i64>) -> Result<u64> {
    let path = paths.attachment(run_id, source);
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let _ = std::fs::remove_file(&path);
    let jira: Option<String> = match scenario_id {
        Some(sid) => c
            .query_row(
                "SELECT jira_key FROM bug_links WHERE scenario_id = ?1 ORDER BY id LIMIT 1",
                [sid],
                |r| r.get(0),
            )
            .ok()
            .or_else(|| {
                c.query_row("SELECT TRIM(jira_keys) FROM scenarios WHERE id = ?1 AND TRIM(jira_keys) <> ''", [sid], |r| r.get::<_, String>(0))
                    .ok()
                    .and_then(|s| s.split_whitespace().next().map(String::from))
            }),
        None => None,
    };
    let reason = match jira {
        Some(k) => format!("Silindi, bkz. {k}"),
        None => "Silindi (disk temizliği)".into(),
    };
    c.execute(
        "UPDATE attachments SET deleted_at = ?2, deleted_reason = ?3, size = COALESCE(size, ?4) WHERE id = ?1",
        params![id, crate::db::now(), reason, size as i64],
    )?;
    Ok(size)
}

/// Temizlik: eşik aşılmışsa en eski koşumdan başlayarak önce videolar, sonra görüntüler.
pub fn cleanup(c: &Connection, paths: &Paths, probe: &dyn DiskProbe, warn_pct: f64, clean_pct: f64) -> Result<CleanupReport> {
    let mut rep = CleanupReport::default();
    if probe.used_pct()? < clean_pct {
        return Ok(rep);
    }
    for kind in ["video/%", "image/%"] {
        let mut st = c.prepare(
            "SELECT id, run_id, source, scenario_id FROM attachments
             WHERE deleted_at IS NULL AND mime LIKE ?1 ORDER BY run_id, id",
        )?;
        let rows: Vec<(i64, i64, String, Option<i64>)> =
            st.query_map([kind], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?.collect::<rusqlite::Result<_>>()?;
        let mut current_run = None;
        for (id, run_id, source, sid) in rows {
            // koşum sınırında doluluğu yeniden ölç
            if current_run != Some(run_id) {
                current_run = Some(run_id);
                if probe.used_pct()? < warn_pct {
                    return Ok(rep);
                }
            }
            rep.freed_bytes += delete_attachment(c, paths, id, run_id, &source, sid)?;
            rep.deleted_files += 1;
        }
    }
    Ok(rep)
}

pub async fn disk_loop(st: crate::AppState) {
    loop {
        let (warn, clean) = (st.cfg.disk_warn_pct, st.cfg.disk_clean_pct);
        let probe = FsProbe(st.paths.root().to_path_buf());
        if let Ok(pct) = probe.used_pct() {
            let mut s = st.disk.0.lock().unwrap();
            let was = s.warning;
            s.used_pct = pct;
            s.warning = pct >= warn;
            if s.warning && !was {
                tracing::warn!(doluluk = pct, esik = warn, "disk doluluk eşiği aşıldı");
            }
        }
        let paths = st.paths.clone();
        let res = st
            .db
            .write(move |c| cleanup(c, &paths, &FsProbe(paths.root().to_path_buf()), warn, clean))
            .await;
        match res {
            Ok(r) if r.deleted_files > 0 => {
                tracing::info!(silinen_dosya = r.deleted_files, bosalan_bayt = r.freed_bytes, "disk temizliği yapıldı");
                st.disk.0.lock().unwrap().last_cleanup = Some(crate::db::now());
            }
            Ok(_) => {}
            Err(e) => tracing::error!(hata = %e, "disk temizliği yapılamadı"),
        }
        tokio::time::sleep(std::time::Duration::from_secs(600)).await;
    }
}
