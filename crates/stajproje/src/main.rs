use anyhow::Context;
use stajproje::{auth, logging, AppState, Config};

fn usage() -> ! {
    eprintln!("Kullanım:\n  stajproje                       sunucuyu başlatır\n  stajproje set-admin-password    admin şifresini belirler (.env içine argon2 hash yazar)\n  stajproje backup                hemen yedek alır");
    std::process::exit(2)
}

fn set_admin_password() -> anyhow::Result<()> {
    let env_path = std::env::var("ENV_FILE").unwrap_or_else(|_| ".env".into());
    let pw = match std::env::var("STAJ_NEW_ADMIN_PASSWORD") {
        Ok(p) if !p.is_empty() => p,
        _ => {
            let a = rpassword::prompt_password("Yeni admin şifresi: ")?;
            let b = rpassword::prompt_password("Tekrar: ")?;
            anyhow::ensure!(a == b, "şifreler aynı değil");
            a
        }
    };
    anyhow::ensure!(pw.chars().count() >= 8, "şifre en az 8 karakter olmalı");
    let hash = auth::hash_password(&pw)?;
    auth::update_env_file(std::path::Path::new(&env_path), "ADMIN_PASSWORD_HASH", &hash)?;
    println!("ADMIN_PASSWORD_HASH {env_path} dosyasına yazıldı. Sunucuyu yeniden başlatın.");
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("serve") => {}
        Some("set-admin-password") => return set_admin_password(),
        Some("backup") => {
            let cfg = Config::from_env()?;
            let paths = stajproje::Paths::new(&cfg.data_dir);
            let db = stajproje::Db::open(&paths, cfg.sqlite_cache_kib, 1)?;
            let p = db.backup(&paths, cfg.backup_keep)?;
            println!("{}", p.display());
            return Ok(());
        }
        _ => usage(),
    }
    let cfg = Config::from_env()?;
    let paths = stajproje::Paths::new(&cfg.data_dir);
    paths.ensure().context("DATA_DIR oluşturulamadı")?;
    let log = logging::init(&paths.logs_dir(), cfg.log_max_days, cfg.log_max_bytes, cfg.log_to_stdout)?;
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .max_blocking_threads(8)
        .enable_all()
        .build()?
        .block_on(async move {
            let bind = cfg.bind;
            let state = AppState::new(cfg, log.recent.clone())?;
            stajproje::spawn_background(&state);
            let app = stajproje::router(state);
            let listener = tokio::net::TcpListener::bind(bind).await.with_context(|| format!("{bind} dinlenemedi"))?;
            tracing::info!(adres = %bind, surum = common::VERSION, "sunucu başladı");
            axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>())
                .with_graceful_shutdown(async {
                    let _ = tokio::signal::ctrl_c().await;
                })
                .await?;
            drop(log);
            anyhow::Ok(())
        })
}
