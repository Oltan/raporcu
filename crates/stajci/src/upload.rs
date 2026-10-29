//! `stajci upload` (SW-2.1, SW-2.6, SW-5.4) ve `stajci finish` (SW-3.5).
//!
//! Sonuçlar zip'lenip gönderilir. Sunucuya ulaşılamazsa artan aralıklarla denenir; yine olmazsa
//! zip `.stajci/queue/` altına yazılır (GitLab artifact olarak saklanır) ve sunucu geri gelince
//! `stajci upload --resume` ile gönderilir.

use crate::client::ensure_ok;
use crate::ctx::{self, say, Ctx, SavedRun};
use anyhow::{Context, Result};
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn zip_results(dir: &Path, out: &Path) -> Result<usize> {
    if let Some(p) = out.parent() {
        std::fs::create_dir_all(p)?;
    }
    let f = std::fs::File::create(out)?;
    let mut w = zip::ZipWriter::new(std::io::BufWriter::new(f));
    let deflate = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let mut n = 0;
    if let Ok(rd) = std::fs::read_dir(dir) {
        let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.is_file()).collect();
        entries.sort();
        for p in entries {
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            // video ve görüntü zaten sıkıştırılmış
            let opts = if name.ends_with(".json") || name.ends_with(".txt") { deflate } else { stored };
            w.start_file(format!("allure-results/{name}"), opts)?;
            let mut src = std::fs::File::open(&p)?;
            std::io::copy(&mut src, &mut w)?;
            n += 1;
        }
    }
    w.finish()?.flush()?;
    Ok(n)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct QueueItem {
    /// sunucudaki hedef yol (`/api/ci/runs/12/upload` ya da `/api/ci/runs?...`)
    path: String,
    zip: String,
    run_id: Option<i64>,
}

fn target_path(c: &Ctx) -> (String, Option<i64>) {
    match c.saved_run() {
        Some(r) if r.run_id > 0 => (format!("/api/ci/runs/{}/upload", r.run_id), Some(r.run_id)),
        _ => (format!("/api/ci/runs?{}", ctx::ci_info(&c.project_dir).to_query()), None),
    }
}

fn on_success(c: &Ctx, body: &str, run_id: Option<i64>) -> Result<()> {
    let v: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
    let id = v["run_id"].as_i64().or(run_id).unwrap_or(0);
    let url = v["report_url"].as_str().unwrap_or("").to_string();
    ctx::set_run_id(id);
    c.save_run(&SavedRun { run_id: id, report_url: url.clone() })?;
    say(format!(
        "yüklendi: {} sonuç ({} tekrar), {} ek",
        v["accepted_results"], v["duplicate_results"], v["attachments"]
    ));
    // SW-5.4: rapor linki iş çıktısında
    say(format!("Rapor: {url}"));
    Ok(())
}

pub fn upload(c: &Ctx) -> Result<i32> {
    if let Some(r) = c.saved_run() {
        ctx::set_run_id(r.run_id);
    }
    if !c.results_dir.is_dir() {
        say(format!(
            "UYARI: sonuç klasörü yok: {}. Testler koşmadı ya da Allure eklentisi eksik olabilir.",
            c.results_dir.display()
        ));
    }
    let zip = c.state_dir.join(format!("sonuclar-{}.zip", uuid::Uuid::new_v4().simple()));
    let n = zip_results(&c.results_dir, &zip)?;
    say(format!("{n} dosya zip'lendi ({} KB)", std::fs::metadata(&zip).map(|m| m.len() / 1024).unwrap_or(0)));
    let (path, run_id) = target_path(c);
    let client = c.client();
    let res = client.with_retry("yükleme", || client.post_file(&path, &zip));
    match res {
        Ok(r) if (200..300).contains(&r.status) => {
            let _ = std::fs::remove_file(&zip);
            on_success(c, &r.body, run_id)?;
            Ok(0)
        }
        Ok(r) => {
            let _ = std::fs::remove_file(&zip);
            let msg = serde_json::from_str::<serde_json::Value>(&r.body).ok().and_then(|v| v["hata"].as_str().map(String::from)).unwrap_or(r.body);
            say(format!("HATA: yükleme reddedildi ({}): {msg}", r.status));
            Ok(1)
        }
        Err(e) => {
            let q = c.queue_dir();
            std::fs::create_dir_all(&q)?;
            let stamp = uuid::Uuid::new_v4().simple().to_string();
            let qzip = q.join(format!("{stamp}.zip"));
            std::fs::rename(&zip, &qzip).or_else(|_| std::fs::copy(&zip, &qzip).map(|_| ()))?;
            let item = QueueItem { path, zip: format!("{stamp}.zip"), run_id };
            std::fs::write(q.join(format!("{stamp}.json")), serde_json::to_string_pretty(&item)?)?;
            say(format!(
                "HATA: sunucuya ulaşılamadı ({e}). Sonuçlar {} içinde saklandı (GitLab artifact). Sunucu geri gelince: stajci upload --resume",
                q.display()
            ));
            Ok(1)
        }
    }
}

/// Kuyruktaki yüklemeleri gönderir. `queue` verilmezse `.stajci/queue`.
pub fn resume(c: &Ctx, queue: Option<PathBuf>) -> Result<i32> {
    let q = queue.unwrap_or_else(|| c.queue_dir());
    let mut items: Vec<PathBuf> = std::fs::read_dir(&q)
        .with_context(|| format!("kuyruk klasörü yok: {}", q.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    items.sort();
    if items.is_empty() {
        say("kuyrukta gönderilecek sonuç yok");
        return Ok(0);
    }
    let client = c.client();
    let mut failed = 0;
    for meta in items {
        let item: QueueItem = serde_json::from_str(&std::fs::read_to_string(&meta)?)?;
        let zip = q.join(&item.zip);
        match client.with_retry("kuyruk gönderimi", || client.post_file(&item.path, &zip)) {
            Ok(r) if (200..300).contains(&r.status) => {
                on_success(c, &r.body, item.run_id)?;
                std::fs::remove_file(&zip)?;
                std::fs::remove_file(&meta)?;
            }
            Ok(r) => {
                say(format!("HATA: kuyruktaki {} reddedildi ({}): {}", item.zip, r.status, r.body));
                failed += 1;
            }
            Err(e) => {
                say(format!("HATA: {} gönderilemedi: {e}", item.zip));
                failed += 1;
            }
        }
    }
    Ok(if failed == 0 { 0 } else { 1 })
}

pub fn finish(c: &Ctx, status: &str) -> Result<i32> {
    let Some(r) = c.saved_run() else {
        say("koşum bilgisi yok (.stajci/run.json); finish atlandı");
        return Ok(0);
    };
    ctx::set_run_id(r.run_id);
    let client = c.client();
    let body = serde_json::json!({ "status": status });
    match client
        .with_retry("koşum bitirme", || client.post_json(&format!("/api/ci/runs/{}/finish", r.run_id), &body))
        .and_then(|x| ensure_ok(x, "koşum bitirme"))
    {
        Ok(x) => {
            let v = x.json().unwrap_or_default();
            say(format!(
                "koşum {}: {} senaryo, {} geçti, {} kaldı. Rapor: {}",
                v["status"].as_str().unwrap_or(status),
                v["total"],
                v["passed"],
                v["failed"].as_i64().unwrap_or(0) + v["broken"].as_i64().unwrap_or(0),
                r.report_url
            ));
            Ok(0)
        }
        Err(e) => {
            say(format!("UYARI: koşum bitirilemedi ({e}); sunucu RUN_STALL_TIMEOUT sonunda kendisi kapatır"));
            Ok(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_2_1_zip_results_contains_all_files() {
        let tmp = tempfile::tempdir().unwrap();
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata/allure/ornek-giris");
        let out = tmp.path().join("x.zip");
        let n = zip_results(&src, &out).unwrap();
        assert_eq!(n, std::fs::read_dir(&src).unwrap().count());
        let za = zip::ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
        assert_eq!(za.len(), n);
    }
}
