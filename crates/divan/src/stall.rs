//! SW-3.5: belirli süre hiç sonuç gelmeyen "çalışıyor" koşumlar interrupted olur.

use crate::{defterdar, AppState};
use anyhow::Result;
use rusqlite::Connection;
use std::time::Duration;

pub fn mark_stalled(c: &Connection, timeout: Duration) -> Result<Vec<i64>> {
    let cutoff = (chrono::Utc::now() - chrono::Duration::from_std(timeout)?).to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let mut st = c.prepare("SELECT id FROM runs WHERE status = 'running' AND last_activity_at <= ?1")?;
    let ids: Vec<i64> = st.query_map([cutoff], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
    for id in &ids {
        defterdar::recompute_run(c, *id)?;
        // recompute last_activity'yi günceller; interrupted durumu kalıcıdır
        defterdar::finish_run(c, *id, "interrupted")?;
    }
    Ok(ids)
}

pub async fn stall_loop(st: AppState) {
    let every = (st.cfg.run_stall_timeout / 10).clamp(Duration::from_secs(1), Duration::from_secs(30));
    loop {
        tokio::time::sleep(every).await;
        let timeout = st.cfg.run_stall_timeout;
        match st.db.write(move |c| mark_stalled(c, timeout)).await {
            Ok(ids) => {
                for id in ids {
                    crate::hooks::after_finish(&st, id).await;
                }
            }
            Err(e) => tracing::error!(hata = %e, "zaman aşımı denetimi yapılamadı"),
        }
    }
}
