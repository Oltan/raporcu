//! SW-3.4: canlı koşum ekranı. Sonuç geldikçe ilgili koşumun abonelerine WebSocket ile anlık
//! görüntü (sayaçlar + senaryo listesi) gönderilir.

use crate::{queries, AppState};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::Response;
use rusqlite::Connection;
use serde::Serialize;
use std::time::Duration;

#[derive(Serialize, Debug, Clone)]
pub struct LiveScenario {
    pub name: String,
    pub feature: Option<String>,
    /// gecti | kaldi | kosuyor | bekliyor | atlandi | bilinmiyor
    pub state: String,
    pub status: Option<String>,
    pub message: Option<String>,
    pub scenario_id: Option<i64>,
    pub screenshot_id: Option<i64>,
    pub attempt: Option<i64>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Snapshot {
    pub run_id: i64,
    pub status: String,
    pub progress: String,
    pub total_expected: Option<i64>,
    pub done: i64,
    pub passed: i64,
    pub failed: i64,
    pub running: i64,
    pub waiting: i64,
    pub scenarios: Vec<LiveScenario>,
    /// Cucumber canlı olay eklentisinden: şu an koşan senaryo ve adımı (eklenti yoksa None)
    pub current: Option<Current>,
}

/// Şu an koşan senaryo (bellekte tutulur; veritabanına yazılmaz, sunucu yeniden başlarsa kaybolur).
#[derive(Serialize, Debug, Clone, Default)]
pub struct Current {
    pub name: String,
    pub feature: String,
    pub uri: String,
    pub line: i64,
    /// senaryo koşuyor mu (false: son senaryo bitti, sıradaki henüz başlamadı)
    pub active: bool,
    pub step: Option<String>,
    pub step_no: i64,
    pub steps: Vec<DoneStep>,
    /// bu koşumda aynı senaryonun kaçıncı başlatılışı (rerun)
    pub attempt: i64,
    pub elapsed_s: u64,
    pub finished_scenarios: i64,
    /// Biten senaryolar (ad, feature, Cucumber durumu, uri|satır): Allure dosyası gelmeden sonucu göstermek için
    #[serde(skip)]
    pub finished: Vec<(String, String, String, String)>,
    #[serde(skip)]
    pub started: Option<std::time::Instant>,
    #[serde(skip)]
    pub seen: std::collections::HashMap<String, i64>,
}

#[derive(Serialize, Debug, Clone)]
pub struct DoneStep {
    pub text: String,
    /// PASSED | FAILED | SKIPPED | PENDING | UNDEFINED | AMBIGUOUS
    pub status: String,
    pub ms: i64,
    pub error: Option<String>,
}

/// Koşum → şu an koşan senaryo
#[derive(Clone, Default)]
pub struct LiveStates(pub std::sync::Arc<std::sync::Mutex<std::collections::HashMap<i64, Current>>>);

impl LiveStates {
    pub fn get(&self, run_id: i64) -> Option<Current> {
        self.0.lock().unwrap().get(&run_id).cloned().map(|mut c| {
            c.elapsed_s = c.started.map(|t| t.elapsed().as_secs()).unwrap_or(0);
            c
        })
    }

    pub fn remove(&self, run_id: i64) {
        self.0.lock().unwrap().remove(&run_id);
    }

    /// CanliOlayEklentisi olaylarını uygular (sırayla). Tanınmayan olaylar yok sayılır.
    pub fn apply(&self, run_id: i64, events: &[serde_json::Value]) {
        let mut m = self.0.lock().unwrap();
        let cur = m.entry(run_id).or_default();
        let s = |v: &serde_json::Value, k: &str| v[k].as_str().unwrap_or("").to_string();
        for e in events {
            match e["tur"].as_str().unwrap_or("") {
                "senaryo_basladi" => {
                    let key = format!("{}|{}", s(e, "uri"), e["satir"].as_i64().unwrap_or(0));
                    let n = cur.seen.entry(key).or_insert(0);
                    *n += 1;
                    let attempt = *n;
                    cur.name = s(e, "ad");
                    cur.feature = s(e, "feature");
                    cur.uri = s(e, "uri");
                    cur.line = e["satir"].as_i64().unwrap_or(0);
                    cur.active = true;
                    cur.step = None;
                    cur.step_no = 0;
                    cur.steps.clear();
                    cur.attempt = attempt;
                    cur.started = Some(std::time::Instant::now());
                }
                "adim_basladi" => {
                    cur.step = Some(crate::logging::truncate(&s(e, "metin"), 300));
                    cur.step_no = e["sira"].as_i64().unwrap_or(cur.step_no + 1);
                }
                "adim_bitti" => {
                    cur.steps.push(DoneStep {
                        text: crate::logging::truncate(&s(e, "metin"), 300),
                        status: s(e, "durum"),
                        ms: e["sure_ms"].as_i64().unwrap_or(0),
                        error: e["hata"].as_str().map(|h| crate::logging::truncate(h, 500)),
                    });
                    cur.step = None;
                    if cur.steps.len() > 100 {
                        cur.steps.remove(0);
                    }
                }
                "senaryo_bitti" => {
                    let key = format!("{}|{}", cur.uri, cur.line);
                    cur.finished.push((cur.name.clone(), cur.feature.clone(), s(e, "durum"), key));
                    cur.active = false;
                    cur.step = None;
                    cur.finished_scenarios += 1;
                }
                _ => {}
            }
        }
    }
}

fn state_of(status: &str) -> &'static str {
    match status {
        "passed" => "gecti",
        "failed" | "broken" => "kaldi",
        "skipped" => "atlandi",
        _ => "bilinmiyor",
    }
}

pub fn snapshot(c: &Connection, run_id: i64, current: Option<Current>) -> anyhow::Result<Option<Snapshot>> {
    let Some(run) = queries::run(c, run_id)? else { return Ok(None) };
    let results = queries::run_scenarios(c, run_id)?;
    let mut expected: Vec<(String, Option<String>)> = {
        let mut st = c.prepare("SELECT name, feature FROM expected_scenarios WHERE run_id = ?1 ORDER BY position")?;
        let rows = st.query_map([run_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    let mut out = Vec::new();
    let mut used = vec![false; results.len()];
    let is_running = run.status == "running";
    // Eklenti olayı varsa koşan senaryo kesin bilinir; yoksa ilk sonuçsuz beklenen senaryo tahmin edilir.
    let active = current.as_ref().filter(|c| c.active && is_running);
    let mut running_marked = current.is_some() && active.is_none();
    // olaylara göre bitmiş ama Allure dosyası henüz gelmemiş senaryolar (son durumları)
    let mut finished: Vec<(String, String, String)> = current.as_ref().map(|c| last_per_scenario(&c.finished)).unwrap_or_default();
    let mut finished_used = finished.clone();
    for (name, feature) in expected.drain(..) {
        let hit = results
            .iter()
            .enumerate()
            .find(|(i, r)| !used[*i] && r.name == name && (feature.is_none() || r.feature == feature));
        match hit {
            Some((i, r)) => {
                used[i] = true;
                out.push(from_result(c, r)?);
            }
            None if take_finished(&mut finished, &name, feature.as_deref()).is_some() => {
                let st = take_finished(&mut finished_used, &name, feature.as_deref()).unwrap_or_default();
                let (state, status) = match st.as_str() {
                    "PASSED" => ("gecti", "passed"),
                    "FAILED" | "AMBIGUOUS" => ("kaldi", "failed"),
                    _ => ("bilinmiyor", "unknown"),
                };
                out.push(LiveScenario {
                    name,
                    feature,
                    state: state.into(),
                    status: Some(status.into()),
                    message: None,
                    scenario_id: None,
                    screenshot_id: None,
                    attempt: None,
                });
            }
            None => {
                let is_current = match active {
                    Some(a) => !running_marked && a.name == name && (feature.is_none() || feature.as_deref() == Some(a.feature.as_str())),
                    None => !running_marked,
                };
                let state = if is_running && is_current {
                    running_marked = true;
                    "kosuyor"
                } else if is_running {
                    "bekliyor"
                } else {
                    "bilinmiyor"
                };
                out.push(LiveScenario {
                    name,
                    feature,
                    state: state.into(),
                    status: None,
                    message: None,
                    scenario_id: None,
                    screenshot_id: None,
                    attempt: None,
                });
            }
        }
    }
    for (i, r) in results.iter().enumerate() {
        if !used[i] {
            out.push(from_result(c, r)?);
        }
    }
    let count = |s: &str| out.iter().filter(|x| x.state == s).count() as i64;
    Ok(Some(Snapshot {
        run_id,
        progress: run.progress(),
        status: run.status.clone(),
        total_expected: run.total_expected,
        done: run.total,
        passed: run.passed,
        failed: run.failures(),
        running: count("kosuyor"),
        waiting: count("bekliyor"),
        scenarios: out,
        current: current.filter(|_| is_running),
    }))
}

/// Rerun'da aynı senaryo (aynı uri|satır) birden çok kez biter; son durum geçerlidir.
/// Senaryo taslağı satırları aynı ada sahiptir ama satırları farklıdır, ayrı kalırlar.
fn last_per_scenario(list: &[(String, String, String, String)]) -> Vec<(String, String, String)> {
    let mut out: Vec<(String, String, String)> = Vec::new();
    let mut keys: Vec<&str> = Vec::new();
    for (n, f, st, k) in list {
        match keys.iter().position(|x| x == k) {
            Some(i) => out[i].2 = st.clone(),
            None => {
                keys.push(k);
                out.push((n.clone(), f.clone(), st.clone()));
            }
        }
    }
    out
}

/// (ad, feature) eşleşen ilk kaydı listeden çıkarır ve durumunu döner.
fn take_finished(list: &mut Vec<(String, String, String)>, name: &str, feature: Option<&str>) -> Option<String> {
    let i = list.iter().position(|(n, f, _)| n == name && feature.is_none_or(|x| x == f))?;
    Some(list.remove(i).2)
}

fn from_result(c: &Connection, r: &queries::ScenarioRow) -> anyhow::Result<LiveScenario> {
    let shot = if r.is_failure() {
        queries::first_screenshot(c, r.id)?.map(|a| a.id)
    } else {
        None
    };
    Ok(LiveScenario {
        name: r.name.clone(),
        feature: r.feature.clone(),
        state: state_of(&r.status).into(),
        status: Some(r.status.clone()),
        message: r.message.as_ref().map(|m| crate::logging::truncate(m, 500)),
        scenario_id: Some(r.id),
        screenshot_id: shot,
        attempt: Some(r.attempt),
    })
}

pub async fn ws(State(st): State<AppState>, Path(run_id): Path<i64>, up: WebSocketUpgrade) -> Response {
    up.on_upgrade(move |sock| serve(st, run_id, sock))
}

async fn send_snapshot(st: &AppState, run_id: i64, sock: &mut WebSocket) -> bool {
    let cur = st.canli.get(run_id);
    match st.db.read(move |c| snapshot(c, run_id, cur)).await {
        Ok(Some(s)) => sock
            .send(Message::Text(serde_json::to_string(&s).unwrap_or_default().into()))
            .await
            .is_ok(),
        _ => false,
    }
}

async fn serve(st: AppState, run_id: i64, mut sock: WebSocket) {
    let mut rx = st.live.subscribe();
    if !send_snapshot(&st, run_id, &mut sock).await {
        return;
    }
    loop {
        tokio::select! {
            m = rx.recv() => match m {
                Ok(id) if id == run_id => {
                    // kısa süre içinde gelen güncellemeleri birleştir
                    tokio::time::sleep(Duration::from_millis(250)).await;
                    while rx.try_recv().is_ok() {}
                    if !send_snapshot(&st, run_id, &mut sock).await {
                        return;
                    }
                }
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    if !send_snapshot(&st, run_id, &mut sock).await {
                        return;
                    }
                }
                Err(_) => return,
            },
            m = sock.recv() => match m {
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => return,
                _ => {}
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_3_4_rerun_last_status_wins_outline_rows_separate() {
        let l = vec![
            ("Yavaş".to_string(), "F".to_string(), "FAILED".to_string(), "a|41".to_string()),
            ("Taslak".into(), "F".into(), "PASSED".into(), "a|23".into()),
            ("Taslak".into(), "F".into(), "FAILED".into(), "a|24".into()),
            ("Yavaş".into(), "F".into(), "PASSED".into(), "a|41".into()),
        ];
        let r = last_per_scenario(&l);
        assert_eq!(r.len(), 3);
        assert_eq!(r[0].2, "PASSED", "rerun'da son durum");
        assert_eq!((r[1].2.as_str(), r[2].2.as_str()), ("PASSED", "FAILED"));
    }

    #[test]
    fn test_sw_3_4_apply_real_events_counts_scenarios() {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/canli/ornek-giris-olaylar.jsonl");
        let ev: Vec<serde_json::Value> = std::fs::read_to_string(p)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        let st = LiveStates::default();
        st.apply(1, &ev);
        let c = st.get(1).unwrap();
        assert!(!c.active);
        assert_eq!(c.finished_scenarios, 7, "örnek koşu rerun'sız: 7 senaryo");
        assert_eq!(last_per_scenario(&c.finished).len(), 7);
    }
}
