//! Yapılandırma: ortam değişkenleri ve `.env` dosyası (SW-10.5).
//!
//! Gizli değerler (tokenlar, şifre hash'i) yalnızca burada okunur ve `Debug` çıktısında
//! gizlenir (SW-9.2).

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JiraMode {
    /// Otomatik bug açma kapalı (eşleşme yine çalışır)
    Off,
    /// Gözlem: Jira'ya yazılmaz, "açılacaktı" kaydı tutulur (SW-6.3)
    Observe,
    On,
}

#[derive(Clone)]
pub struct Secret(String);

impl Secret {
    pub fn new(s: impl Into<String>) -> Self {
        Secret(s.into())
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("***")
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub data_dir: PathBuf,
    pub bind: SocketAddr,
    /// Linklerde kullanılan dış adres, örn. `http://stajproje.sirket.local:8080`
    pub public_url: String,

    pub ci_api_token: Option<Secret>,
    pub admin_password_hash: Option<Secret>,
    pub gitlab_webhook_token: Option<Secret>,
    pub gitlab_url: Option<String>,
    pub gitlab_token: Option<Secret>,

    pub max_upload_bytes: u64,
    pub max_entry_bytes: u64,
    pub max_zip_entries: usize,
    pub memory_target_mb: u64,
    pub sqlite_cache_kib: i64,
    pub read_pool_size: usize,

    pub run_stall_timeout: Duration,
    pub backup_hour: u32,
    pub backup_keep: usize,
    pub log_max_days: usize,
    pub log_max_bytes: u64,
    pub log_to_stdout: bool,

    pub disk_warn_pct: f64,
    pub disk_clean_pct: f64,

    pub jira_url: Option<String>,
    pub jira_user: Option<String>,
    pub jira_token: Option<Secret>,
    pub jira_project: Option<String>,
    pub jira_issue_type: String,
    pub jira_mode: JiraMode,
    pub jira_max_per_run: usize,
    pub jira_require_retry: bool,

    pub smtp_host: Option<String>,
    pub smtp_port: u16,
    pub mail_from: String,
    pub mail_to: Vec<String>,
    pub dry_run: bool,

    /// OpenCode triage kararının admin onayı olmadan geçerli olduğu en düşük güven (SW-4.5)
    pub triage_auto_confidence: f64,
    pub flaky_window: usize,

    /// stajci ikililerinin bulunduğu klasör (SW-10.9). Varsayılan DATA_DIR/tools
    pub stajci_dir: Option<PathBuf>,
}

fn var(name: &str) -> Option<String> {
    std::env::var(name).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

fn parse<T: std::str::FromStr>(name: &str, default: T) -> anyhow::Result<T> {
    match var(name) {
        None => Ok(default),
        Some(v) => v
            .parse()
            .map_err(|_| anyhow::anyhow!("{name} değeri geçersiz: {v}")),
    }
}

impl Config {
    /// `.env` (varsa) yüklenir, sonra ortam değişkenleri okunur. Ortam değişkeni `.env`'i ezer.
    pub fn from_env() -> anyhow::Result<Config> {
        let _ = dotenvy::dotenv();
        let data_dir = PathBuf::from(var("DATA_DIR").unwrap_or_else(|| "data".into()));
        let bind: SocketAddr = parse("BIND_ADDR", "0.0.0.0:8080".parse().unwrap())?;
        let public_url = var("PUBLIC_URL").unwrap_or_else(|| format!("http://127.0.0.1:{}", bind.port()));
        let jira_mode = match var("JIRA_AUTO_CREATE").as_deref() {
            None | Some("observe") | Some("gozlem") => JiraMode::Observe,
            Some("on") | Some("acik") => JiraMode::On,
            Some("off") | Some("kapali") => JiraMode::Off,
            Some(v) => anyhow::bail!("JIRA_AUTO_CREATE geçersiz: {v} (off|observe|on)"),
        };
        Ok(Config {
            data_dir,
            bind,
            public_url: public_url.trim_end_matches('/').to_string(),
            ci_api_token: var("CI_API_TOKEN").map(Secret),
            admin_password_hash: var("ADMIN_PASSWORD_HASH").map(Secret),
            gitlab_webhook_token: var("GITLAB_WEBHOOK_TOKEN").map(Secret),
            gitlab_url: var("GITLAB_URL").map(|u| u.trim_end_matches('/').to_string()),
            gitlab_token: var("GITLAB_TOKEN").map(Secret),
            max_upload_bytes: parse::<u64>("MAX_UPLOAD_MB", 512)? * 1024 * 1024,
            max_entry_bytes: parse::<u64>("MAX_ENTRY_MB", 256)? * 1024 * 1024,
            max_zip_entries: parse("MAX_ZIP_ENTRIES", 50_000)?,
            memory_target_mb: parse("MEMORY_TARGET_MB", 100)?,
            sqlite_cache_kib: parse("SQLITE_CACHE_KIB", 8192)?,
            read_pool_size: parse("DB_READ_POOL", 4)?,
            run_stall_timeout: Duration::from_secs(parse("RUN_STALL_TIMEOUT_SECS", 900)?),
            backup_hour: parse("BACKUP_HOUR", 3)?,
            backup_keep: parse("BACKUP_KEEP", 7)?,
            log_max_days: parse("LOG_MAX_DAYS", 14)?,
            log_max_bytes: parse::<u64>("LOG_MAX_MB", 500)? * 1024 * 1024,
            log_to_stdout: parse("LOG_STDOUT", true)?,
            disk_warn_pct: parse("DISK_WARN_PCT", 80.0)?,
            disk_clean_pct: parse("DISK_CLEAN_PCT", 90.0)?,
            jira_url: var("JIRA_URL").map(|u| u.trim_end_matches('/').to_string()),
            jira_user: var("JIRA_USER"),
            jira_token: var("JIRA_TOKEN").map(Secret),
            jira_project: var("JIRA_PROJECT"),
            jira_issue_type: var("JIRA_ISSUE_TYPE").unwrap_or_else(|| "Bug".into()),
            jira_mode,
            jira_max_per_run: parse("JIRA_MAX_BUGS_PER_RUN", 3)?,
            jira_require_retry: parse("JIRA_REQUIRE_RETRY", true)?,
            smtp_host: var("SMTP_HOST"),
            smtp_port: parse("SMTP_PORT", 25)?,
            mail_from: var("MAIL_FROM").unwrap_or_else(|| "stajproje@localhost".into()),
            mail_to: var("MAIL_TO")
                .map(|v| v.split([',', ';']).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
                .unwrap_or_default(),
            dry_run: parse("DRY_RUN", false)?,
            triage_auto_confidence: parse("TRIAGE_AUTO_CONFIDENCE", 0.7)?,
            flaky_window: parse("FLAKY_WINDOW", 10)?,
            stajci_dir: var("STAJCI_DIR").map(PathBuf::from),
        })
    }

    /// Testler için: geçici DATA_DIR, sabit tokenlar, tüm dış servisler kapalı.
    pub fn for_tests(data_dir: impl Into<PathBuf>) -> Config {
        Config {
            data_dir: data_dir.into(),
            bind: "127.0.0.1:0".parse().unwrap(),
            public_url: "http://stajproje.test".into(),
            ci_api_token: Some(Secret::new("test-ci-token")),
            admin_password_hash: None,
            gitlab_webhook_token: Some(Secret::new("test-webhook-token")),
            gitlab_url: None,
            gitlab_token: None,
            max_upload_bytes: 512 * 1024 * 1024,
            max_entry_bytes: 256 * 1024 * 1024,
            max_zip_entries: 50_000,
            memory_target_mb: 100,
            sqlite_cache_kib: 2048,
            read_pool_size: 2,
            run_stall_timeout: Duration::from_secs(900),
            backup_hour: 3,
            backup_keep: 7,
            log_max_days: 14,
            log_max_bytes: 500 * 1024 * 1024,
            log_to_stdout: false,
            disk_warn_pct: 80.0,
            disk_clean_pct: 90.0,
            jira_url: None,
            jira_user: None,
            jira_token: None,
            jira_project: None,
            jira_issue_type: "Bug".into(),
            jira_mode: JiraMode::Observe,
            jira_max_per_run: 3,
            jira_require_retry: true,
            smtp_host: None,
            smtp_port: 25,
            mail_from: "stajproje@test".into(),
            mail_to: vec![],
            dry_run: true,
            triage_auto_confidence: 0.7,
            flaky_window: 10,
            stajci_dir: None,
        }
    }

    /// Hata mesajlarından ve loglardan çıkarılacak gizli değerler (SW-9.2).
    pub fn secrets(&self) -> Vec<String> {
        [&self.ci_api_token, &self.gitlab_token, &self.gitlab_webhook_token, &self.jira_token, &self.admin_password_hash]
            .into_iter()
            .flatten()
            .map(|s| s.expose().to_string())
            .filter(|s| s.len() >= 4)
            .collect()
    }
}

/// Metindeki gizli değerleri `***` ile değiştirir (SW-9.2).
pub fn redact(text: &str, secrets: &[String]) -> String {
    let mut out = text.to_string();
    for s in secrets {
        if !s.is_empty() {
            out = out.replace(s.as_str(), "***");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_9_2_secret_debug_and_redact() {
        let mut cfg = Config::for_tests("/tmp/x");
        cfg.gitlab_token = Some(Secret::new("glpat-COKGIZLI"));
        let dbg = format!("{cfg:?}");
        assert!(!dbg.contains("glpat-COKGIZLI"));
        let msg = redact("https://gitlab/api?private_token=glpat-COKGIZLI hata", &cfg.secrets());
        assert!(!msg.contains("glpat-COKGIZLI"));
        assert!(msg.contains("***"));
    }
}
