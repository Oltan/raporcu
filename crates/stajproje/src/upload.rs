//! SW-2.1, SW-2.3, SW-10.6: Allure zip yükleme.
//!
//! Gövde belleğe alınmaz: önce sınır kontrolüyle diske yazılır, sonra zip dosya dosya işlenir.
//! JSON dosyaları tek tek okunur; ekler doğrudan koşumun dosya klasörüne kopyalanır.
//! Zip güvenliği: toplam boyut sınırı, giriş sayısı sınırı, açılmış boyut sınırı, `..` ya da mutlak
//! yol içeren girişler bütün yüklemenin reddine yol açar.

use crate::ingest::{self, RunCtx};
use crate::paths::Paths;
use axum::body::Body;
use axum::http::StatusCode;
use common::allure::{self, FileKind};
use common::api::{CiInfo, IngestSummary};
use futures_util::StreamExt;
use rusqlite::Connection;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

pub const MAX_JSON_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug)]
pub struct UploadError {
    pub status: StatusCode,
    pub reason: String,
}

impl UploadError {
    pub fn new(status: StatusCode, reason: impl Into<String>) -> Self {
        UploadError { status, reason: reason.into() }
    }
}

impl From<anyhow::Error> for UploadError {
    fn from(e: anyhow::Error) -> Self {
        tracing::error!(hata = %e, "yükleme işlenemedi");
        UploadError::new(StatusCode::INTERNAL_SERVER_ERROR, "sunucu hatası")
    }
}

impl From<std::io::Error> for UploadError {
    fn from(e: std::io::Error) -> Self {
        UploadError::from(anyhow::Error::from(e))
    }
}

pub const NO_RESULTS_MESSAGE: &str = "Yüklemede hiç Allure sonuç dosyası (*-result.json) yok. Test projesinde Allure eklentisi \
(allure-cucumber7-jvm) eksik ya da Cucumber plugin ayarına eklenmemiş olabilir: \
cucumber.plugin=io.qameta.allure.cucumber7jvm.AllureCucumber7Jvm";

#[derive(Clone, Copy)]
pub struct Limits {
    pub max_upload: u64,
    pub max_entry: u64,
    pub max_entries: usize,
}

/// Gövdeyi `tmp/` altına yazar; sınır aşılırsa 413.
pub async fn body_to_file(body: Body, tmp_dir: &Path, max: u64) -> Result<(PathBuf, u64), UploadError> {
    tokio::fs::create_dir_all(tmp_dir).await?;
    let path = tmp_dir.join(format!("upload-{}.zip", uuid::Uuid::new_v4().simple()));
    let mut f = tokio::fs::File::create(&path).await?;
    let mut stream = body.into_data_stream();
    let mut total: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(c) => c,
            Err(e) => {
                let _ = tokio::fs::remove_file(&path).await;
                return Err(UploadError::new(StatusCode::BAD_REQUEST, format!("gövde okunamadı: {e}")));
            }
        };
        total += chunk.len() as u64;
        if total > max {
            drop(f);
            let _ = tokio::fs::remove_file(&path).await;
            return Err(UploadError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                format!("yükleme boyutu sınırı aşıldı ({} MB)", max / 1024 / 1024),
            ));
        }
        f.write_all(&chunk).await?;
    }
    f.flush().await?;
    Ok((path, total))
}

/// Zip içindeki bir giriş adını doğrular; dosya adını (son bileşen) döner. Klasörler için None.
pub fn validate_entry_name(name: &str) -> Result<Option<String>, UploadError> {
    let bad = || UploadError::new(StatusCode::BAD_REQUEST, format!("zip içinde güvensiz yol reddedildi: {name}"));
    if name.starts_with('/') || name.starts_with('\\') || name.contains(':') || name.contains('\0') {
        return Err(bad());
    }
    let parts: Vec<&str> = name.split(['/', '\\']).collect();
    if parts.iter().any(|p| *p == "..") {
        return Err(bad());
    }
    match parts.last() {
        Some(last) if !last.is_empty() => Ok(Some(last.to_string())),
        _ => Ok(None),
    }
}

pub enum Target {
    New(CiInfo),
    Existing(i64),
}

/// Zip dosyasını işler. Bloklayan bağlamda (spawn_blocking / Db::write) çağrılır.
pub fn process_zip(
    conn: &mut Connection,
    paths: &Paths,
    limits: Limits,
    zip_path: &Path,
    target: Target,
) -> Result<(IngestSummary, RunCtx, bool), UploadError> {
    let file = std::fs::File::open(zip_path)?;
    let mut za = zip::ZipArchive::new(file)
        .map_err(|e| UploadError::new(StatusCode::BAD_REQUEST, format!("geçerli bir zip değil: {e}")))?;
    if za.len() > limits.max_entries {
        return Err(UploadError::new(StatusCode::BAD_REQUEST, format!("zip'te çok fazla dosya var ({})", za.len())));
    }
    // 1. geçiş: adlar ve boyutlar (içerik açılmadan)
    let mut total_uncompressed: u64 = 0;
    let mut result_count = 0;
    for i in 0..za.len() {
        let e = za
            .by_index_raw(i)
            .map_err(|e| UploadError::new(StatusCode::BAD_REQUEST, format!("zip girişi okunamadı: {e}")))?;
        let raw_name = e.name().map_err(|_| UploadError::new(StatusCode::BAD_REQUEST, "zip giriş adı okunamadı"))?.to_string();
        let Some(base) = validate_entry_name(&raw_name)? else { continue };
        if e.size() > limits.max_entry {
            return Err(UploadError::new(StatusCode::PAYLOAD_TOO_LARGE, format!("zip girişi çok büyük: {base}")));
        }
        total_uncompressed += e.size();
        if allure::file_kind(&base) == FileKind::Result {
            result_count += 1;
        }
    }
    if total_uncompressed > limits.max_upload.saturating_mul(4) {
        return Err(UploadError::new(StatusCode::PAYLOAD_TOO_LARGE, "zip açılmış boyutu sınırı aşıyor"));
    }
    if result_count == 0 {
        return Err(UploadError::new(StatusCode::UNPROCESSABLE_ENTITY, NO_RESULTS_MESSAGE));
    }

    let tx = conn.transaction().map_err(anyhow::Error::from)?;
    let (run, created) = match &target {
        Target::New(ci) => {
            let pid = ingest::ensure_project(&tx, ci).map_err(|e| UploadError::new(StatusCode::BAD_REQUEST, e.to_string()))?;
            (ingest::create_run(&tx, pid, ci, "running", None)?, true)
        }
        Target::Existing(id) => (
            ingest::run_ctx(&tx, *id)?.ok_or_else(|| UploadError::new(StatusCode::NOT_FOUND, "koşum bulunamadı"))?,
            false,
        ),
    };
    let files_dir = paths.run_files_dir(run.run_id);
    std::fs::create_dir_all(&files_dir)?;
    let mut summary = IngestSummary { run_id: run.run_id, ..Default::default() };

    let res: Result<(), UploadError> = (|| {
        for i in 0..za.len() {
            let mut e = za
                .by_index(i)
                .map_err(|e| UploadError::new(StatusCode::BAD_REQUEST, format!("zip girişi açılamadı: {e}")))?;
            let raw_name = e.name().map_err(|_| UploadError::new(StatusCode::BAD_REQUEST, "zip giriş adı okunamadı"))?.to_string();
            let Some(base) = validate_entry_name(&raw_name)? else { continue };
            match allure::file_kind(&base) {
                FileKind::Result | FileKind::Container => {
                    let mut buf = Vec::new();
                    (&mut e).take(MAX_JSON_BYTES + 1).read_to_end(&mut buf)?;
                    if buf.len() as u64 > MAX_JSON_BYTES {
                        return Err(UploadError::new(StatusCode::PAYLOAD_TOO_LARGE, format!("json çok büyük: {base}")));
                    }
                    ingest_json(&tx, &run, &base, &buf, &mut summary)?;
                }
                FileKind::Attachment => {
                    if !allure::is_safe_file_name(&base) {
                        continue;
                    }
                    let dest = files_dir.join(&base);
                    if dest.exists() {
                        continue;
                    }
                    let tmp = files_dir.join(format!(".{base}.part"));
                    let mut out = std::fs::File::create(&tmp)?;
                    let n = std::io::copy(&mut (&mut e).take(limits.max_entry + 1), &mut out)?;
                    out.flush()?;
                    drop(out);
                    if n > limits.max_entry {
                        let _ = std::fs::remove_file(&tmp);
                        return Err(UploadError::new(StatusCode::PAYLOAD_TOO_LARGE, format!("ek çok büyük: {base}")));
                    }
                    std::fs::rename(&tmp, &dest)?;
                    summary.attachments += 1;
                }
                FileKind::Other => {}
            }
        }
        ingest::link_fixture_attachments(&tx, run.run_id)?;
        ingest::recompute_run(&tx, run.run_id)?;
        if created {
            ingest::finish_run(&tx, run.run_id, "finished")?;
        }
        Ok(())
    })();
    match res {
        Ok(()) => {
            tx.commit().map_err(anyhow::Error::from)?;
            Ok((summary, run, created))
        }
        Err(e) => {
            drop(tx);
            if created {
                let _ = std::fs::remove_dir_all(paths.run_dir(run.run_id));
            }
            Err(e)
        }
    }
}

/// Tek bir result/container json'unu işler (zip ve artımlı yükleme ortak yolu).
pub fn ingest_json(c: &Connection, run: &RunCtx, name: &str, bytes: &[u8], summary: &mut IngestSummary) -> Result<(), UploadError> {
    match allure::file_kind(name) {
        FileKind::Result => match allure::parse_result(bytes) {
            Ok(r) => match ingest::ingest_result(c, run, &r)? {
                Some(_) => summary.accepted_results += 1,
                None => summary.duplicate_results += 1,
            },
            Err(e) => tracing::warn!(run_id = run.run_id, dosya = name, hata = %e, "sonuç dosyası ayrıştırılamadı, atlandı"),
        },
        FileKind::Container => match allure::parse_container(bytes) {
            Ok(ct) => {
                if ingest::ingest_container(c, run.run_id, &ct)? {
                    summary.containers += 1;
                }
            }
            Err(e) => tracing::warn!(run_id = run.run_id, dosya = name, hata = %e, "container dosyası ayrıştırılamadı, atlandı"),
        },
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_2_1_zip_entry_names() {
        assert_eq!(validate_entry_name("allure-results/a-result.json").unwrap().as_deref(), Some("a-result.json"));
        assert_eq!(validate_entry_name("allure-results/").unwrap(), None);
        for bad in ["../x-result.json", "a/../../x", "/etc/passwd", "C:\\x", "a\\..\\b"] {
            assert!(validate_entry_name(bad).is_err(), "{bad}");
        }
    }
}
