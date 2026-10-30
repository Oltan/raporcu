//! raporcu: test sonuç merkezi.

pub mod auth;
pub mod charts;
pub mod config;
pub mod db;
pub mod disk;
pub mod email;
pub mod export;
pub mod fix;
pub mod gitlab;
pub mod history;
pub mod hooks;
pub mod ingest;
pub mod jira;
pub mod live;
pub mod logging;
pub mod paths;
pub mod queries;
pub mod share;
pub mod stall;
pub mod triage;
pub mod upload;
pub mod web;

use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;

pub use config::Config;
pub use db::Db;
pub use paths::Paths;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<Config>,
    pub paths: Paths,
    pub db: Db,
    pub auth: Arc<dyn auth::Authenticator>,
    pub sessions: auth::Sessions,
    pub throttle: auth::LoginThrottle,
    pub recent_logs: logging::RecentLogs,
    /// Sonucu değişen koşumun id'si (SW-3.4)
    pub live: tokio::sync::broadcast::Sender<i64>,
    pub disk: disk::DiskState,
    /// Şu an koşan senaryo/adım (Cucumber canlı olay eklentisi)
    pub canli: live::LiveStates,
}

impl AppState {
    pub fn new(cfg: Config, recent_logs: logging::RecentLogs) -> anyhow::Result<AppState> {
        let paths = Paths::new(&cfg.data_dir);
        let db = Db::open(&paths, cfg.sqlite_cache_kib, cfg.read_pool_size)?;
        let auth = Arc::new(auth::LocalAdmin::new(
            cfg.admin_password_hash.as_ref().map(|s| s.expose().to_string()),
        ));
        Ok(AppState {
            cfg: Arc::new(cfg),
            paths,
            db,
            auth,
            sessions: Default::default(),
            throttle: Default::default(),
            recent_logs,
            live: tokio::sync::broadcast::channel(256).0,
            disk: Default::default(),
            canli: Default::default(),
        })
    }
}

pub fn router(state: AppState) -> Router {
    let ci = Router::new()
        .route("/ping", get(|| async { "ok" }))
        .route("/runs", post(web::ci::upload_new))
        .route("/runs/start", post(web::ci::start))
        .route("/runs/{id}/upload", post(web::ci::upload_existing))
        .route("/runs/{id}/results", post(web::ci::result_file))
        .route("/runs/{id}/events", post(web::ci::live_events))
        .route("/runs/{id}/finish", post(web::ci::finish))
        .route("/runs/{id}/fix-candidates", get(web::ci::fix_candidates))
        .route("/runs/{id}/fix-result", post(web::ci::fix_result))
        .route("/runs/{id}/fix-started", post(web::ci::fix_started))
        .layer(axum::extract::DefaultBodyLimit::max(state.cfg.max_entry_bytes as usize))
        .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth::ci_guard));
    Router::new()
        .route("/api/health", get(web::health))
        .route("/api/gitlab/webhook", post(web::gitlab_hook::webhook))
        .nest("/api/ci", ci)
        .merge(web::routes())
        .with_state(state)
}

/// Arka plan görevleri: gecelik yedek, log temizliği, ...
pub fn spawn_background(state: &AppState) {
    tokio::spawn(stall::stall_loop(state.clone()));
    tokio::spawn(disk::disk_loop(state.clone()));
    tokio::spawn(db::backup_loop(
        state.db.clone(),
        state.paths.clone(),
        state.cfg.backup_hour,
        state.cfg.backup_keep,
    ));
    let (dir, days, bytes) = (state.paths.logs_dir(), state.cfg.log_max_days, state.cfg.log_max_bytes);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            let (n, freed) = logging::enforce_retention(&dir, days, bytes);
            if n > 0 {
                tracing::info!(silinen_dosya = n, bosalan_bayt = freed, "log temizliği yapıldı");
            }
        }
    });
}
