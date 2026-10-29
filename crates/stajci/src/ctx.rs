//! Ortak bağlam: sunucu adresi, token, durum klasörü, GitLab CI değişkenleri, çıktı satırları.

use common::api::CiInfo;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};

static RUN_ID: AtomicI64 = AtomicI64::new(0);

pub fn set_run_id(id: i64) {
    RUN_ID.store(id, Ordering::Relaxed);
}

/// SW-10.7: CI çıktısına kısa satırlar; her satırda aynı run_id.
pub fn say(msg: impl AsRef<str>) {
    let id = RUN_ID.load(Ordering::Relaxed);
    if id > 0 {
        println!("[stajci run_id={id}] {}", msg.as_ref());
    } else {
        println!("[stajci] {}", msg.as_ref());
    }
}

pub fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

pub fn env_or(name: &str, default: &str) -> String {
    env(name).unwrap_or_else(|| default.to_string())
}

pub struct Ctx {
    pub server: String,
    pub token: String,
    pub project_dir: PathBuf,
    pub state_dir: PathBuf,
    pub results_dir: PathBuf,
}

impl Ctx {
    pub fn from_env() -> Ctx {
        let project_dir = env("STAJ_PROJECT_DIR").map(PathBuf::from).unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| ".".into()));
        let state_dir = env("STAJ_STATE_DIR").map(PathBuf::from).unwrap_or_else(|| project_dir.join(".stajci"));
        let results_dir = env("STAJ_RESULTS_DIR")
            .map(|p| {
                let p = PathBuf::from(p);
                if p.is_absolute() {
                    p
                } else {
                    project_dir.join(p)
                }
            })
            .unwrap_or_else(|| project_dir.join("target").join("allure-results"));
        Ctx {
            server: env_or("STAJ_SERVER", "http://127.0.0.1:8080"),
            token: env("CI_API_TOKEN").unwrap_or_default(),
            project_dir,
            state_dir,
            results_dir,
        }
    }

    pub fn client(&self) -> crate::client::Client {
        crate::client::Client::new(&self.server, &self.token)
    }

    pub fn run_file(&self) -> PathBuf {
        self.state_dir.join("run.json")
    }
    pub fn stop_file(&self) -> PathBuf {
        self.state_dir.join("stop")
    }
    pub fn queue_dir(&self) -> PathBuf {
        self.state_dir.join("queue")
    }
    pub fn dotenv_file(&self) -> PathBuf {
        self.project_dir.join("stajci.env")
    }

    pub fn saved_run(&self) -> Option<SavedRun> {
        let s = std::fs::read_to_string(self.run_file()).ok()?;
        serde_json::from_str(&s).ok()
    }

    pub fn save_run(&self, r: &SavedRun) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.state_dir)?;
        std::fs::write(self.run_file(), serde_json::to_string_pretty(r)?)?;
        write_dotenv(&self.dotenv_file(), r)?;
        Ok(())
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct SavedRun {
    pub run_id: i64,
    pub report_url: String,
}

/// GitLab `artifacts:reports:dotenv` dosyası: sonraki işler ve environment linki için (SW-5.4).
pub fn write_dotenv(path: &Path, r: &SavedRun) -> anyhow::Result<()> {
    std::fs::write(path, format!("STAJ_RUN_ID={}\nSTAJ_REPORT_URL={}\n", r.run_id, r.report_url))?;
    Ok(())
}

pub fn os_name() -> String {
    env("STAJ_OS").unwrap_or_else(|| if cfg!(windows) { "windows".into() } else { std::env::consts::OS.into() })
}

/// GitLab CI değişkenlerinden koşum bilgisi (SW-4.3, SW-4.6).
pub fn ci_info(project_dir: &Path) -> CiInfo {
    CiInfo {
        project_id: env("CI_PROJECT_ID"),
        project_path: env("CI_PROJECT_PATH").or_else(|| env("STAJ_PROJECT")).unwrap_or_else(|| {
            project_dir.file_name().map(|n| format!("yerel/{}", n.to_string_lossy())).unwrap_or_else(|| "yerel/proje".into())
        }),
        project_url: env("CI_PROJECT_URL"),
        branch: env("CI_COMMIT_REF_NAME").or_else(|| env("STAJ_BRANCH")).unwrap_or_else(|| "yerel".into()),
        commit: env("CI_COMMIT_SHA"),
        pipeline_id: env("CI_PIPELINE_ID"),
        pipeline_url: env("CI_PIPELINE_URL"),
        job_id: env("CI_JOB_ID"),
        job_url: env("CI_JOB_URL"),
        os: Some(os_name()),
    }
}

/// Basit komut satırı ayırıcı (kabuk kullanılmaz): boşluk, tek/çift tırnak.
pub fn split_command(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut has = false;
    for ch in s.chars() {
        match (quote, ch) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => cur.push(c),
            (None, '"') | (None, '\'') => {
                quote = Some(ch);
                has = true;
            }
            (None, c) if c.is_whitespace() => {
                if has || !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                    has = false;
                }
            }
            (None, c) => cur.push(c),
        }
    }
    if has || !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_1_1_split_command_without_shell() {
        assert_eq!(split_command("mvn -B test"), vec!["mvn", "-B", "test"]);
        assert_eq!(split_command(r#"a "b c" 'd e' f"#), vec!["a", "b c", "d e", "f"]);
        assert_eq!(split_command(r#"x """#), vec!["x", ""]);
    }
}
