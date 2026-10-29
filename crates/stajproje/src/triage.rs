//! SW-4.5: başarısız senaryonun nedenine karar (taslak).
//!
//! Karar kaynakları ve önceliği: admin > OpenCode > kural. Her karar kaynağıyla birlikte
//! `triage_decisions` tablosunda saklanır; eski kararlar silinmez, geçmişte kalır.
//!
//! Açık soru 3 için varsayılanlar (docs/KARARLAR.md):
//! - Kategoriler: flaky, test_kodu, urun, ortam, bilinmiyor
//! - OpenCode kararı güveni `TRIAGE_AUTO_CONFIDENCE` (0.7) ve üstündeyse kendiliğinden geçerli;
//!   altındaysa öneri olarak gösterilir, geçerli karar kural kararı kalır.

use crate::db::now;
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

pub const CATEGORIES: &[(&str, &str)] = &[
    ("flaky", "Flaky"),
    ("test_kodu", "Test kodu hatası"),
    ("urun", "Ürün hatası"),
    ("ortam", "Ortam sorunu"),
    ("bilinmiyor", "Bilinmiyor"),
];

pub fn label(cat: &str) -> &'static str {
    CATEGORIES.iter().find(|(k, _)| *k == cat).map(|(_, v)| *v).unwrap_or("Bilinmiyor")
}

pub fn is_category(cat: &str) -> bool {
    CATEGORIES.iter().any(|(k, _)| *k == cat)
}

/// Bilinen ortam hata kalıpları (zaman aşımı, bağlantı hatası, ekran bulunamadı).
pub const ENV_PATTERNS: &[&str] = &[
    "TimeoutException",
    "timed out",
    "Timed out",
    "zaman aşımı",
    "ConnectException",
    "Connection refused",
    "UnknownHostException",
    "NoRouteToHostException",
    "SocketTimeoutException",
    "Can't connect to X11",
    "cannot open display",
    "Unable to open DISPLAY",
    "HeadlessException",
    "No X11 DISPLAY",
    "Failed to initialize the display",
];

#[derive(Debug, Clone, Serialize)]
pub struct Decision {
    pub id: i64,
    pub scenario_id: i64,
    pub category: String,
    pub source: String,
    pub reason: String,
    pub confidence: Option<f64>,
    pub created_at: String,
}

impl Decision {
    pub fn label(&self) -> &'static str {
        label(&self.category)
    }
    pub fn source_label(&self) -> &'static str {
        match self.source.as_str() {
            "admin" => "Admin",
            "opencode" => "OpenCode",
            _ => "Kural",
        }
    }
    pub fn confidence_pct(&self) -> String {
        self.confidence
            .map(|c| format!("%{}", (c * 100.0).round() as i64))
            .unwrap_or_default()
    }
}

/// Kural kararı: flaky geçmişi → flaky; ortam kalıbı → ortam; broken → test kodu; failed → ürün.
pub fn rule_decision(status: &str, message: Option<&str>, trace: Option<&str>, flaky: bool) -> (String, String) {
    if flaky {
        return ("flaky".into(), "Son koşumlarda hem geçti hem kaldı (flaky geçmişi)".into());
    }
    let text = format!("{}\n{}", message.unwrap_or(""), trace.unwrap_or(""));
    if let Some(p) = ENV_PATTERNS.iter().find(|p| text.contains(**p)) {
        return ("ortam".into(), format!("Bilinen ortam hata kalıbı: {p}"));
    }
    match status {
        "broken" => ("test_kodu".into(), "Allure durumu broken: test kodunda beklenmeyen istisna".into()),
        "failed" => ("urun".into(), "Allure durumu failed: doğrulama (assertion) başarısız".into()),
        _ => ("bilinmiyor".into(), format!("Allure durumu {status}")),
    }
}

pub fn add(c: &Connection, scenario_id: i64, category: &str, source: &str, reason: &str, confidence: Option<f64>) -> Result<i64> {
    anyhow::ensure!(is_category(category), "geçersiz kategori: {category}");
    c.execute(
        "INSERT INTO triage_decisions (scenario_id, category, source, reason, confidence, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            scenario_id,
            category,
            source,
            crate::logging::truncate(reason, 2000),
            confidence,
            now()
        ],
    )?;
    let id = c.last_insert_rowid();
    tracing::info!(senaryo_id = scenario_id, kategori = category, kaynak = source, "triage kararı");
    Ok(id)
}

fn dec_row(r: &rusqlite::Row) -> rusqlite::Result<Decision> {
    Ok(Decision {
        id: r.get(0)?,
        scenario_id: r.get(1)?,
        category: r.get(2)?,
        source: r.get(3)?,
        reason: r.get(4)?,
        confidence: r.get(5)?,
        created_at: r.get(6)?,
    })
}

const COLS: &str = "id, scenario_id, category, source, reason, confidence, created_at";

pub fn history(c: &Connection, scenario_id: i64) -> Result<Vec<Decision>> {
    let mut st = c.prepare(&format!(
        "SELECT {COLS} FROM triage_decisions WHERE scenario_id = ?1 ORDER BY id DESC"
    ))?;
    let rows = st.query_map([scenario_id], dec_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Geçerli karar: admin > OpenCode (güven eşiği üstü) > kural; her kaynakta en yenisi.
pub fn current(c: &Connection, scenario_id: i64, auto_conf: f64) -> Result<Option<Decision>> {
    for (src, min) in [("admin", None), ("opencode", Some(auto_conf)), ("kural", None)] {
        let d = c
            .query_row(
                &format!(
                    "SELECT {COLS} FROM triage_decisions WHERE scenario_id = ?1 AND source = ?2 AND (?3 IS NULL OR COALESCE(confidence, 0) >= ?3)
                     ORDER BY id DESC LIMIT 1"
                ),
                params![scenario_id, src, min],
                dec_row,
            )
            .optional()?;
        if d.is_some() {
            return Ok(d);
        }
    }
    Ok(None)
}

/// Geçerli kararı senaryo satırına yazar (arama için).
pub fn refresh(c: &Connection, scenario_id: i64, auto_conf: f64) -> Result<Option<Decision>> {
    let d = current(c, scenario_id, auto_conf)?;
    c.execute(
        "UPDATE scenarios SET triage_category = ?2, triage_source = ?3 WHERE id = ?1",
        params![
            scenario_id,
            d.as_ref().map(|d| d.category.clone()),
            d.as_ref().map(|d| d.source.clone())
        ],
    )?;
    Ok(d)
}

/// Koşum bitince: kararı olmayan her başarısız son deneme için kural kararı.
pub fn apply_rules(c: &Connection, run_id: i64, window: usize, auto_conf: f64) -> Result<usize> {
    let (project_id, branch): (i64, String) = c.query_row("SELECT project_id, branch FROM runs WHERE id = ?1", [run_id], |r| {
        Ok((r.get(0)?, r.get(1)?))
    })?;
    let flaky = crate::history::flaky_keys(c, project_id, &branch, window)?;
    let mut st = c.prepare(
        "SELECT id, scenario_key, status, message, trace FROM scenarios s
         WHERE run_id = ?1 AND is_latest = 1 AND status IN ('failed', 'broken')
           AND NOT EXISTS (SELECT 1 FROM triage_decisions d WHERE d.scenario_id = s.id AND d.source = 'kural')",
    )?;
    let rows: Vec<(i64, String, String, Option<String>, Option<String>)> = st
        .query_map([run_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?
        .collect::<rusqlite::Result<_>>()?;
    for (id, key, status, msg, trace) in &rows {
        let (cat, reason) = rule_decision(status, msg.as_deref(), trace.as_deref(), flaky.contains(key));
        add(c, *id, &cat, "kural", &reason, None)?;
        refresh(c, *id, auto_conf)?;
    }
    Ok(rows.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_4_5_rule_categories() {
        assert_eq!(rule_decision("failed", Some("expected x"), None, true).0, "flaky");
        assert_eq!(
            rule_decision("failed", Some("java.util.concurrent.TimeoutException: 10s"), None, false).0,
            "ortam"
        );
        assert_eq!(
            rule_decision("broken", None, Some("java.net.ConnectException: Connection refused"), false).0,
            "ortam"
        );
        assert_eq!(rule_decision("broken", Some("NullPointerException"), None, false).0, "test_kodu");
        assert_eq!(rule_decision("failed", Some("expected: <a> but was: <b>"), None, false).0, "urun");
        assert_eq!(rule_decision("unknown", None, None, false).0, "bilinmiyor");
    }
}
