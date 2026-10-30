//! SW-6.1, 6.2, 6.3: Jira (Server/Data Center REST v2) istemcisi ve otomatik eşleşme.
//!
//! - Bug açılırken DOORS numarası hem label hem açıklamada; Jira label'ı reddederse label'sız açılır.
//! - Bug açılırken ya da senaryo var olan bir bug'a bağlanırken ekran görüntüsü issue'ya eklenir.
//! - Otomatik bug açma: koşum başına üst sınır, aynı hata için tek bug, retry sonrasında,
//!   varsayılan gözlem modu (Jira'ya yazılmaz, kayıt tutulur).

use crate::config::{redact, Config, JiraMode};
use crate::db::now;
use crate::paths::Paths;
use anyhow::{bail, Result};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::io::Read;

pub struct Jira {
    base: String,
    user: Option<String>,
    token: String,
    project: String,
    issue_type: String,
    secrets: Vec<String>,
    agent: ureq::Agent,
}

#[derive(Debug, Clone)]
pub struct NewBug {
    pub summary: String,
    pub description: String,
    pub labels: Vec<String>,
}

impl Jira {
    pub fn from_config(cfg: &Config) -> Option<Jira> {
        Some(Jira {
            base: cfg.jira_url.clone()?,
            user: cfg.jira_user.clone(),
            token: cfg.jira_token.as_ref()?.expose().to_string(),
            project: cfg.jira_project.clone()?,
            issue_type: cfg.jira_issue_type.clone(),
            secrets: cfg.secrets(),
            agent: crate::gitlab::agent(),
        })
    }

    fn auth(&self) -> String {
        match &self.user {
            // Basic: kullanıcı + API token/şifre
            Some(u) => format!("Basic {}", base64(&format!("{u}:{}", self.token))),
            // Personal Access Token
            None => format!("Bearer {}", self.token),
        }
    }

    fn call(&self, method: &str, path: &str, body: Option<&Value>) -> Result<(u16, Value)> {
        let url = format!("{}{path}", self.base);
        let res = match (method, body) {
            ("GET", _) => self
                .agent
                .get(&url)
                .header("Authorization", &self.auth())
                .header("Accept", "application/json")
                .call(),
            (_, b) => self
                .agent
                .post(&url)
                .header("Authorization", &self.auth())
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .send(b.map(|v| v.to_string()).unwrap_or_default()),
        };
        let mut r = res.map_err(|e| anyhow::anyhow!("Jira'ya ulaşılamadı: {}", redact(&e.to_string(), &self.secrets)))?;
        let st = r.status().as_u16();
        let mut s = String::new();
        r.body_mut().as_reader().take(4 << 20).read_to_string(&mut s)?;
        Ok((
            st,
            serde_json::from_str(&s).unwrap_or(Value::String(crate::logging::truncate(&s, 500))),
        ))
    }

    /// SW-6.1: bug açar; label reddedilirse label'sız tekrar dener.
    pub fn create_bug(&self, bug: &NewBug) -> Result<String> {
        let mut fields = json!({
            "project": { "key": self.project },
            "issuetype": { "name": self.issue_type },
            "summary": crate::logging::truncate(&bug.summary, 250),
            "description": bug.description,
            "labels": bug.labels,
        });
        let (mut st, mut v) = self.call("POST", "/rest/api/2/issue", Some(&json!({ "fields": fields })))?;
        if st == 400 && v["errors"].get("labels").is_some() {
            tracing::warn!(servis = "jira", durum = st, deneme = 1, "Jira label'ı reddetti, label'sız açılıyor");
            fields.as_object_mut().unwrap().remove("labels");
            (st, v) = self.call("POST", "/rest/api/2/issue", Some(&json!({ "fields": fields })))?;
        }
        if !(200..300).contains(&st) {
            tracing::warn!(servis = "jira", durum = st, deneme = 2, "Jira bug açılamadı");
            bail!(
                "Jira {st}: {}",
                redact(&crate::logging::truncate(&v.to_string(), 300), &self.secrets)
            );
        }
        v["key"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("Jira yanıtında key yok"))
    }

    /// Ekran görüntüsünü issue'ya ekler.
    pub fn attach(&self, key: &str, file_name: &str, data: &[u8]) -> Result<()> {
        let boundary = format!("raporcu{}", uuid::Uuid::new_v4().simple());
        let mut body = Vec::with_capacity(data.len() + 256);
        body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{file_name}\"\r\nContent-Type: application/octet-stream\r\n\r\n").as_bytes());
        body.extend_from_slice(data);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        let mut r = self
            .agent
            .post(&format!("{}/rest/api/2/issue/{key}/attachments", self.base))
            .header("Authorization", &self.auth())
            .header("X-Atlassian-Token", "no-check")
            .header("Content-Type", &format!("multipart/form-data; boundary={boundary}"))
            .send(&body[..])
            .map_err(|e| anyhow::anyhow!("Jira'ya ulaşılamadı: {}", redact(&e.to_string(), &self.secrets)))?;
        let st = r.status().as_u16();
        let mut s = String::new();
        let _ = r.body_mut().as_reader().take(1 << 16).read_to_string(&mut s);
        if !(200..300).contains(&st) {
            tracing::warn!(servis = "jira", durum = st, deneme = 1, "Jira'ya ek yüklenemedi");
            bail!("Jira ek {st}");
        }
        Ok(())
    }

    /// Açık bug arama: DOORS label'ı ya da senaryo adı ile.
    pub fn find_open_bug(&self, doors: &[String], scenario: &str) -> Result<Option<String>> {
        let mut parts = Vec::new();
        if !doors.is_empty() {
            parts.push(format!(
                "labels in ({})",
                doors.iter().map(|d| format!("\"{d}\"")).collect::<Vec<_>>().join(",")
            ));
        }
        let name = scenario.replace(['"', '\\'], " ");
        parts.push(format!("summary ~ \"\\\"{name}\\\"\""));
        let jql = format!(
            "project = \"{}\" AND statusCategory != Done AND ({}) ORDER BY created DESC",
            self.project,
            parts.join(" OR ")
        );
        let (st, v) = self.call(
            "GET",
            &format!("/rest/api/2/search?maxResults=1&fields=key&jql={}", urlencoding::encode(&jql)),
            None,
        )?;
        if !(200..300).contains(&st) {
            tracing::warn!(servis = "jira", durum = st, deneme = 1, "Jira araması başarısız");
            bail!("Jira arama {st}");
        }
        Ok(v["issues"][0]["key"].as_str().map(String::from))
    }
}

fn base64(s: &str) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let b = s.as_bytes();
    let mut out = String::new();
    for ch in b.chunks(3) {
        let n = (ch[0] as u32) << 16 | (*ch.get(1).unwrap_or(&0) as u32) << 8 | *ch.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= ch.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Aynı hatanın parmak izi: proje + hata mesajının ilk satırı (sayılar ve adresler atılır).
pub fn fingerprint(project_id: i64, message: &str) -> String {
    use sha2::{Digest, Sha256};
    let first = message.lines().next().unwrap_or("");
    let norm: String = first.chars().map(|c| if c.is_ascii_digit() { '#' } else { c }).collect();
    let norm = norm.split('@').next().unwrap_or("").trim().to_lowercase();
    hex::encode(&Sha256::digest(format!("{project_id}|{norm}"))[..12])
}

pub fn bug_text(run_url: &str, scenario: &str, feature: &str, doors: &[String], message: &str, steps: &[String]) -> NewBug {
    let mut d = String::new();
    d.push_str(&format!("Otomatik test senaryosu kaldı: *{scenario}*\n\nFeature: {feature}\n"));
    if !doors.is_empty() {
        d.push_str(&format!("DOORS: {}\n", doors.join(", ")));
    }
    d.push_str(&format!(
        "Rapor: {run_url}\n\n*Hata*\n{{noformat}}\n{}\n{{noformat}}\n",
        crate::logging::truncate(message, 2000)
    ));
    if !steps.is_empty() {
        d.push_str("\n*Adımlar*\n");
        for s in steps {
            d.push_str(&format!("# {s}\n"));
        }
    }
    NewBug {
        summary: format!(
            "[Otomasyon] {scenario}: {}",
            crate::logging::truncate(message.lines().next().unwrap_or(""), 120)
        ),
        description: d,
        labels: doors.to_vec(),
    }
}

struct Failed {
    id: i64,
    project_id: i64,
    key: String,
    name: String,
    feature: String,
    doors: Vec<String>,
    jira_tags: Vec<String>,
    message: String,
    attempt: i64,
    category: Option<String>,
}

fn failed_scenarios(c: &Connection, run_id: i64) -> Result<Vec<Failed>> {
    let mut st = c.prepare(
        "SELECT id, project_id, scenario_key, name, COALESCE(feature, ''), doors_ids, jira_keys, COALESCE(message, ''), attempt, triage_category
         FROM scenarios WHERE run_id = ?1 AND is_latest = 1 AND status = 'failed' ORDER BY id",
    )?;
    let rows = st.query_map([run_id], |r| {
        Ok(Failed {
            id: r.get(0)?,
            project_id: r.get(1)?,
            key: r.get(2)?,
            name: r.get(3)?,
            feature: r.get(4)?,
            doors: r.get::<_, String>(5)?.split_whitespace().map(String::from).collect(),
            jira_tags: r.get::<_, String>(6)?.split_whitespace().map(String::from).collect(),
            message: r.get(7)?,
            attempt: r.get(8)?,
            category: r.get(9)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn link(c: &Connection, f: &Failed, key: &str, source: &str, fp: Option<&str>) -> Result<bool> {
    Ok(c.execute(
        "INSERT OR IGNORE INTO bug_links (scenario_id, project_id, scenario_key, jira_key, source, fingerprint, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![f.id, f.project_id, f.key, key, source, fp, now()],
    )? > 0)
}

fn record(c: &Connection, run_id: i64, sid: Option<i64>, action: &str, fp: Option<&str>, key: Option<&str>, detail: &str) -> Result<()> {
    c.execute(
        "INSERT INTO jira_actions (run_id, scenario_id, action, fingerprint, jira_key, detail, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![run_id, sid, action, fp, key, crate::logging::truncate(detail, 500), now()],
    )?;
    Ok(())
}

fn screenshot(c: &Connection, paths: &Paths, sid: i64) -> Option<(String, Vec<u8>)> {
    let a = crate::queries::first_screenshot(c, sid).ok()??;
    let data = std::fs::read(paths.attachment(a.run_id, &a.source)).ok()?;
    Some((format!("ekran-{sid}.png"), data))
}

#[derive(Debug, Default, PartialEq)]
pub struct MatchReport {
    pub linked_by_tag: usize,
    pub matched: usize,
    pub created: usize,
    pub would_create: usize,
    pub skipped_cap: usize,
}

/// SW-6.2 + SW-6.3: koşum bittikten sonra çalışır. `jira` None ise yalnızca tag eşleşmesi yapılır.
pub fn process_run(c: &Connection, paths: &Paths, cfg: &Config, jira: Option<&Jira>, run_id: i64) -> Result<MatchReport> {
    let mut rep = MatchReport::default();
    let run_url = format!("{}/runs/{run_id}", cfg.public_url);
    let failed = failed_scenarios(c, run_id)?;
    let mut created_this_run = 0usize;
    for f in &failed {
        // 1) @JIRA-ABC-12 tag'i: doğrudan bağla
        let mut has = false;
        for k in &f.jira_tags {
            if link(c, f, k, "tag", None)? {
                rep.linked_by_tag += 1;
                tracing::info!(run_id, senaryo_id = f.id, issue = %k, "Jira bug'ı eşleşti");
            }
            has = true;
        }
        let existing: i64 = c.query_row("SELECT COUNT(*) FROM bug_links WHERE scenario_id = ?1", [f.id], |r| r.get(0))?;
        if has || existing > 0 {
            continue;
        }
        let fp = fingerprint(f.project_id, &f.message);
        // aynı hata için daha önce açılmış/bağlanmış bug
        let prev: Option<String> = c
            .query_row(
                "SELECT jira_key FROM bug_links WHERE project_id = ?1 AND fingerprint = ?2 ORDER BY id DESC LIMIT 1",
                params![f.project_id, fp],
                |r| r.get(0),
            )
            .ok();
        if let Some(k) = prev {
            link(c, f, &k, "eslesme", Some(&fp))?;
            record(c, run_id, Some(f.id), "eslesti", Some(&fp), Some(&k), "aynı hata parmak izi")?;
            rep.matched += 1;
            continue;
        }
        let Some(j) = jira else { continue };
        // 2) Jira'da açık bug ara
        match j.find_open_bug(&f.doors, &f.name) {
            Ok(Some(k)) => {
                link(c, f, &k, "eslesme", Some(&fp))?;
                record(c, run_id, Some(f.id), "eslesti", Some(&fp), Some(&k), "Jira araması")?;
                tracing::info!(run_id, senaryo_id = f.id, issue = %k, "Jira bug'ı eşleşti");
                if let Some((n, d)) = screenshot(c, paths, f.id) {
                    if let Err(e) = j.attach(&k, &n, &d) {
                        tracing::warn!(servis = "jira", hata = %e, "ekran görüntüsü eklenemedi");
                    }
                }
                rep.matched += 1;
                continue;
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(servis = "jira", run_id, hata = %e, "Jira eşleşmesi yapılamadı");
                continue;
            }
        }
        // 3) Otomatik bug açma (SW-6.3)
        if cfg.jira_mode == JiraMode::Off {
            continue;
        }
        if f.category.as_deref().is_some_and(|c| c != "urun") {
            continue; // yalnızca ürün hatası (triage yoksa failed = ürün)
        }
        if cfg.jira_require_retry && f.attempt < 2 {
            continue; // retry sonrasında
        }
        let dup: bool = c.query_row(
            "SELECT EXISTS(SELECT 1 FROM jira_actions WHERE fingerprint = ?1 AND action IN ('acildi', 'acilacakti') AND run_id = ?2)",
            params![fp, run_id],
            |r| r.get(0),
        )?;
        if dup {
            continue; // aynı koşumda aynı hata için tek bug
        }
        if created_this_run >= cfg.jira_max_per_run {
            rep.skipped_cap += 1;
            continue;
        }
        created_this_run += 1;
        let steps: Vec<String> = crate::queries::steps(c, f.id)?
            .into_iter()
            .filter(|s| s.depth == 0)
            .map(|s| s.name)
            .collect();
        let bug = bug_text(&run_url, &f.name, &f.feature, &f.doors, &f.message, &steps);
        if cfg.jira_mode == JiraMode::Observe {
            record(c, run_id, Some(f.id), "acilacakti", Some(&fp), None, &bug.summary)?;
            tracing::info!(run_id, senaryo_id = f.id, ozet = %bug.summary, "Jira bug'ı açılacaktı (gözlem modu)");
            rep.would_create += 1;
            continue;
        }
        match j.create_bug(&bug) {
            Ok(k) => {
                link(c, f, &k, "otomatik", Some(&fp))?;
                record(c, run_id, Some(f.id), "acildi", Some(&fp), Some(&k), &bug.summary)?;
                tracing::info!(run_id, senaryo_id = f.id, issue = %k, "Jira bug'ı açıldı");
                if let Some((n, d)) = screenshot(c, paths, f.id) {
                    if let Err(e) = j.attach(&k, &n, &d) {
                        tracing::warn!(servis = "jira", hata = %e, "ekran görüntüsü eklenemedi");
                    }
                }
                rep.created += 1;
            }
            Err(e) => {
                record(c, run_id, Some(f.id), "hata", Some(&fp), None, &e.to_string())?;
            }
        }
    }
    Ok(rep)
}

/// Admin düğmesi: tek senaryo için bug aç (gözlem modundan bağımsız).
pub fn create_for_scenario(c: &Connection, paths: &Paths, cfg: &Config, j: &Jira, sid: i64) -> Result<String> {
    let s = crate::queries::scenario(c, sid)?.ok_or_else(|| anyhow::anyhow!("senaryo yok"))?;
    let steps: Vec<String> = crate::queries::steps(c, sid)?
        .into_iter()
        .filter(|s| s.depth == 0)
        .map(|s| s.name)
        .collect();
    let bug = bug_text(
        &format!("{}/runs/{}", cfg.public_url, s.run_id),
        &s.name,
        s.feature.as_deref().unwrap_or(""),
        &s.doors_ids,
        s.message.as_deref().unwrap_or(""),
        &steps,
    );
    let k = j.create_bug(&bug)?;
    let fp = fingerprint(s.project_id, s.message.as_deref().unwrap_or(""));
    c.execute(
        "INSERT OR IGNORE INTO bug_links (scenario_id, project_id, scenario_key, jira_key, source, fingerprint, created_at) VALUES (?1, ?2, ?3, ?4, 'admin', ?5, ?6)",
        params![sid, s.project_id, s.scenario_key, k, fp, now()],
    )?;
    record(c, s.run_id, Some(sid), "acildi", Some(&fp), Some(&k), &bug.summary)?;
    tracing::info!(run_id = s.run_id, senaryo_id = sid, issue = %k, "Jira bug'ı açıldı");
    if let Some((n, d)) = screenshot(c, paths, sid) {
        let _ = j.attach(&k, &n, &d);
    }
    Ok(k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_6_1_base64_and_fingerprint() {
        assert_eq!(base64("kullanici:sifre"), "a3VsbGFuaWNpOnNpZnJl");
        assert_eq!(base64("ab"), "YWI=");
        assert_eq!(fingerprint(1, "Timeout after 30 s\nat x"), fingerprint(1, "Timeout after 45 s"));
        assert_ne!(fingerprint(1, "a"), fingerprint(2, "a"));
    }

    #[test]
    fn test_sw_6_1_doors_in_labels_and_description() {
        let b = bug_text(
            "http://r/runs/1",
            "Giriş",
            "F",
            &["DOORS-1003".into()],
            "expected x",
            &["adım 1".into()],
        );
        assert_eq!(b.labels, vec!["DOORS-1003"]);
        assert!(b.description.contains("DOORS-1003"));
    }
}
