//! `stajci watch` (SW-3.2): allure-results klasörünü izler, her yeni dosyayı hemen gönderir.
//!
//! Test işinden ayrı bir süreçtir; çökerse test etkilenmez. Gönderilemeyen dosyalar bir sonraki
//! turda yeniden denenir; iş sonunda hepsi toplu zip ile de gider (SW-2.6, tekrarlar sayılmaz).

use crate::ctx::{env, say, Ctx};
use common::allure::{file_kind, FileKind};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant, SystemTime};

pub fn watch(c: &Ctx, run_id: i64) -> i32 {
    crate::ctx::set_run_id(run_id);
    let client = c.client();
    let interval = Duration::from_millis(env("STAJ_WATCH_INTERVAL_MS").and_then(|v| v.parse().ok()).unwrap_or(1000));
    let max_life = Duration::from_secs(env("STAJ_WATCH_MAX_SECS").and_then(|v| v.parse().ok()).unwrap_or(6 * 3600));
    let sent_path = c.state_dir.join("sent.txt");
    let mut sent: HashSet<String> = std::fs::read_to_string(&sent_path).unwrap_or_default().lines().map(String::from).collect();
    let mut sizes: HashMap<String, u64> = HashMap::new();
    let started = Instant::now();
    let mut backoff_until = Instant::now();
    let mut backoff = Duration::from_secs(1);
    say("izleyici başladı");
    loop {
        let stopping = c.stop_file().exists() || started.elapsed() > max_life;
        if Instant::now() >= backoff_until {
            let mut failed = false;
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
                    let fresh = meta.modified().ok().and_then(|m| SystemTime::now().duration_since(m).ok()).is_some_and(|age| age < Duration::from_millis(300));
                    let prev = sizes.insert(name.clone(), meta.len());
                    if fresh && prev != Some(meta.len()) && !stopping {
                        continue;
                    }
                    let Ok(data) = std::fs::read(e.path()) else { continue };
                    let path = format!("/api/ci/runs/{run_id}/results?name={name}");
                    match client.post_bytes(&path, "application/octet-stream", &data) {
                        Ok(r) if (200..300).contains(&r.status) => {
                            sent.insert(name.clone());
                            let _ = std::fs::OpenOptions::new().create(true).append(true).open(&sent_path).and_then(|mut f| {
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
