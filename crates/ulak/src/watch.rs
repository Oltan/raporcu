//! `ulak watch` (SW-3.2): allure-results klasörünü izler, her yeni dosyayı hemen gönderir.
//!
//! Test işinden ayrı bir süreçtir; çökerse test etkilenmez. Gönderilemeyen dosyalar bir sonraki
//! turda yeniden denenir; iş sonunda hepsi toplu zip ile de gider (SW-2.6, tekrarlar sayılmaz).
//!
//! Ayrıca Cucumber canlı olay dosyasını (senaryo/adım başladı-bitti, JSON lines) kaldığı yerden
//! okuyup `/events` ucuna gönderir; canlı ekran "şu an koşan senaryo ve adım"ı buradan gösterir.

use crate::ctx::{env, say, Ctx};
use common::allure::{file_kind, FileKind};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant, SystemTime};

pub fn watch(c: &Ctx, run_id: i64) -> i32 {
    crate::ctx::set_run_id(run_id);
    let client = c.client();
    let interval = Duration::from_millis(env("ULAK_WATCH_INTERVAL_MS").and_then(|v| v.parse().ok()).unwrap_or(1000));
    let max_life = Duration::from_secs(env("ULAK_WATCH_MAX_SECS").and_then(|v| v.parse().ok()).unwrap_or(6 * 3600));
    let sent_path = c.state_dir.join("sent.txt");
    let mut sent: HashSet<String> = std::fs::read_to_string(&sent_path)
        .unwrap_or_default()
        .lines()
        .map(String::from)
        .collect();
    let mut sizes: HashMap<String, u64> = HashMap::new();
    let started = Instant::now();
    let mut backoff_until = Instant::now();
    let mut backoff = Duration::from_secs(1);
    say("izleyici başladı");
    let mut events = EventTail {
        path: c.live_events_file(),
        offset: 0,
    };
    loop {
        let stopping = c.stop_file().exists() || started.elapsed() > max_life;
        if Instant::now() >= backoff_until {
            let mut failed = !events.send(&client, run_id);
            if let Ok(rd) = std::fs::read_dir(&c.results_dir) {
                let mut entries: Vec<_> = rd.flatten().collect();
                // ekler önce: rapor senaryoyu ekleriyle birlikte gösterir
                entries.sort_by_key(|e| {
                    let n = e.file_name().to_string_lossy().to_string();
                    (file_kind(&n) != FileKind::Attachment, n)
                });
                for e in entries {
                    let name = e.file_name().to_string_lossy().to_string();
                    if sent.contains(&name) || file_kind(&name) == FileKind::Other {
                        continue;
                    }
                    let Ok(meta) = e.metadata() else { continue };
                    // yazımı süren dosyayı gönderme: boyut iki tur aynı kalmalı ya da dosya 300 ms'den eski olmalı
                    let fresh = meta
                        .modified()
                        .ok()
                        .and_then(|m| SystemTime::now().duration_since(m).ok())
                        .is_some_and(|age| age < Duration::from_millis(300));
                    let prev = sizes.insert(name.clone(), meta.len());
                    if fresh && prev != Some(meta.len()) && !stopping {
                        continue;
                    }
                    let Ok(data) = std::fs::read(e.path()) else { continue };
                    let path = format!("/api/ci/runs/{run_id}/results?name={name}");
                    match client.post_bytes(&path, "application/octet-stream", &data) {
                        Ok(r) if (200..300).contains(&r.status) => {
                            sent.insert(name.clone());
                            let _ = std::fs::OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(&sent_path)
                                .and_then(|mut f| {
                                    use std::io::Write;
                                    writeln!(f, "{name}")
                                });
                        }
                        Ok(r) if r.status < 500 => {
                            say(format!("{name} reddedildi ({}): {}", r.status, r.body));
                            sent.insert(name.clone());
                        }
                        Ok(r) => {
                            say(format!("{name} gönderilemedi ({}), tekrar denenecek", r.status));
                            failed = true;
                            break;
                        }
                        Err(e) => {
                            say(format!("sunucuya ulaşılamadı ({e}); tekrar denenecek"));
                            failed = true;
                            break;
                        }
                    }
                }
            }
            if failed {
                backoff_until = Instant::now() + backoff;
                backoff = (backoff * 2).min(Duration::from_secs(30));
                if stopping {
                    say("iş bitti, gönderilemeyen dosyalar toplu yüklemeyle gönderilecek");
                    return 0;
                }
            } else {
                backoff = Duration::from_secs(1);
                if stopping {
                    say(format!("izleyici bitti: {} dosya gönderildi", sent.len()));
                    return 0;
                }
            }
        }
        std::thread::sleep(interval);
    }
}

/// Canlı olay dosyasını kaldığı yerden okur. Yalnızca tamamlanmış (\n ile biten) satırlar gider.
struct EventTail {
    path: std::path::PathBuf,
    offset: u64,
}

impl EventTail {
    /// Yeni olayları gönderir. Sunucuya ulaşılamazsa false (konum ilerlemez, sonra tekrar).
    fn send(&mut self, client: &crate::client::Client, run_id: i64) -> bool {
        use std::io::{Read, Seek, SeekFrom};
        let Ok(mut f) = std::fs::File::open(&self.path) else { return true };
        let len = f.metadata().map(|m| m.len()).unwrap_or(0);
        if len < self.offset {
            self.offset = 0; // dosya yeniden oluşturulmuş
        }
        if len == self.offset || f.seek(SeekFrom::Start(self.offset)).is_err() {
            return true;
        }
        let mut buf = Vec::new();
        if (&mut f).take(1024 * 1024).read_to_end(&mut buf).is_err() {
            return true;
        }
        let Some(last_nl) = buf.iter().rposition(|b| *b == b'\n') else {
            return true;
        };
        let complete = &buf[..=last_nl];
        let lines: Vec<serde_json::Value> = String::from_utf8_lossy(complete)
            .lines()
            .filter_map(|l| serde_json::from_str(l.trim()).ok())
            .collect();
        if !lines.is_empty() {
            match client.post_json(&format!("/api/ci/runs/{run_id}/events"), &serde_json::Value::Array(lines)) {
                Ok(r) if r.status < 500 => {}
                _ => return false,
            }
        }
        self.offset += complete.len() as u64;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_3_2_event_tail_sends_only_complete_lines() {
        // gönderim olmadan: sunucu yok → false, konum ilerlemez
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("olaylar.jsonl");
        std::fs::write(&p, "{\"tur\":\"senaryo_basladi\"}\n{\"tur\":\"adim_bas").unwrap();
        let client = crate::client::Client::new("http://127.0.0.1:9", "x");
        let mut t = EventTail {
            path: p.clone(),
            offset: 0,
        };
        assert!(!t.send(&client, 1));
        assert_eq!(t.offset, 0);
        // boş/eksik satır: gönderilecek bir şey yok
        std::fs::write(&p, "{\"tur\":\"adim_bas").unwrap();
        assert!(t.send(&client, 1));
        assert_eq!(t.offset, 0);
    }
}
