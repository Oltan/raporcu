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
}

fn state_of(status: &str) -> &'static str {
    match status {
        "passed" => "gecti",
        "failed" | "broken" => "kaldi",
        "skipped" => "atlandi",
        _ => "bilinmiyor",
    }
}

pub fn snapshot(c: &Connection, run_id: i64) -> anyhow::Result<Option<Snapshot>> {
    let Some(run) = queries::run(c, run_id)? else { return Ok(None) };
    let results = queries::run_scenarios(c, run_id)?;
    let mut expected: Vec<(String, Option<String>)> = {
        let mut st = c.prepare("SELECT name, feature FROM expected_scenarios WHERE run_id = ?1 ORDER BY position")?;
        let rows = st.query_map([run_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    let mut out = Vec::new();
    let mut used = vec![false; results.len()];
    let mut running_marked = false;
    let is_running = run.status == "running";
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
            None => {
                let state = if is_running && !running_marked {
                    running_marked = true;
                    "kosuyor"
                } else if is_running {
                    "bekliyor"
                } else {
                    "bilinmiyor"
                };
                out.push(LiveScenario { name, feature, state: state.into(), status: None, message: None, scenario_id: None, screenshot_id: None, attempt: None });
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
    }))
}

fn from_result(c: &Connection, r: &queries::ScenarioRow) -> anyhow::Result<LiveScenario> {
    let shot = if r.is_failure() { queries::first_screenshot(c, r.id)?.map(|a| a.id) } else { None };
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
    match st.db.read(move |c| snapshot(c, run_id)).await {
        Ok(Some(s)) => sock.send(Message::Text(serde_json::to_string(&s).unwrap_or_default().into())).await.is_ok(),
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
