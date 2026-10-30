//! Sonuç geldikten ve koşum bittikten sonra çalışan işler.

use crate::AppState;

/// Yeni sonuç geldi: canlı ekran güncellenir (SW-3.4).
pub async fn after_results(st: &AppState, run_id: i64) {
    let _ = st.live.send(run_id);
}

/// Koşum bitti: triage kuralları (SW-4.5), Jira eşleşmesi (SW-6.2, 6.3).
pub async fn after_finish(st: &AppState, run_id: i64) {
    st.canli.remove(run_id);
    let _ = st.live.send(run_id);
    let (window, conf) = (st.cfg.flaky_window, st.cfg.triage_auto_confidence);
    if let Err(e) = st.db.write(move |c| crate::triage::apply_rules(c, run_id, window, conf)).await {
        tracing::error!(run_id, hata = %e, "triage kuralları uygulanamadı");
    }
    // SW-6.2, 6.3: Jira eşleşmesi (ağ çağrıları içerdiği için arka planda)
    let st2 = st.clone();
    tokio::spawn(async move {
        let (paths, cfg) = (st2.paths.clone(), st2.cfg.clone());
        let res = st2
            .db
            .write(move |c| {
                let j = crate::jira::Jira::from_config(&cfg);
                crate::jira::process_run(c, &paths, &cfg, j.as_ref(), run_id)
            })
            .await;
        if let Err(e) = res {
            tracing::warn!(run_id, hata = %e, "Jira eşleşmesi yapılamadı");
        }
    });
}
