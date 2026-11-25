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
        let (paths, cfg, sablon) = (st2.paths.clone(), st2.cfg.clone(), st2.ayarlar.get().jira);
        let res = st2
            .db
            .write(move |c| {
                let j = crate::jira::Jira::from_config(&cfg);
                crate::jira::process_run(c, &paths, &cfg, &sablon, j.as_ref(), run_id)
            })
            .await;
        if let Err(e) = res {
            tracing::warn!(run_id, hata = %e, "Jira eşleşmesi yapılamadı");
        }
        refresh_jira_issues(&st2, run_id).await;
    });
}

/// SW-6.2: koşumdaki Jira anahtarlarının (tag + bağlı bug) başlık ve durumu Jira'dan çekilir.
/// Veritabanı kilidi tutulmadan: önce anahtarlar okunur, ağ çağrıları ayrı iş parçacığında, sonra yazılır.
pub async fn refresh_jira_issues(st: &AppState, run_id: i64) {
    let Some(j) = crate::jira::Jira::from_config(&st.cfg) else { return };
    let keys = match st.db.read(move |c| crate::jira::issue_keys_to_refresh(c, run_id, 30, 200)).await {
        Ok(k) if !k.is_empty() => k,
        Ok(_) => return,
        Err(e) => {
            tracing::warn!(run_id, hata = %e, "Jira issue anahtarları okunamadı");
            return;
        }
    };
    let n = keys.len();
    let sonuc = tokio::task::spawn_blocking(move || {
        let mut out = Vec::new();
        for k in keys {
            match j.get_issue(&k) {
                Ok(i) => out.push(i),
                Err(e) => {
                    // token reddi ya da Jira'ya ulaşılamıyor: kalanlar da olmaz
                    tracing::warn!(servis = "jira", hata = %e, "Jira issue bilgisi çekilemedi");
                    break;
                }
            }
        }
        out
    })
    .await
    .unwrap_or_default();
    let alinan = sonuc.len();
    if let Err(e) = st
        .db
        .write(move |c| {
            for i in &sonuc {
                crate::jira::save_issue(c, i)?;
            }
            Ok(())
        })
        .await
    {
        tracing::warn!(run_id, hata = %e, "Jira issue bilgisi kaydedilemedi");
    }
    tracing::info!(run_id, istenen = n, alinan, "Jira issue bilgileri yenilendi");
}
