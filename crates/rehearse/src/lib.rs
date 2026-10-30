//! Prova ve smoke için ortak düzenek: sunucuyu ayrı süreç olarak açar, ulak'ı çalıştırır.

use anyhow::{bail, Context, Result};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub const TOKEN: &str = "prova-ci-token";

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

/// Derlenmiş ikililer bu programın yanında (target/<profil>/).
pub fn bin_dir() -> PathBuf {
    std::env::current_exe().unwrap().parent().unwrap().to_path_buf()
}

pub fn exe(name: &str) -> PathBuf {
    bin_dir().join(format!("{name}{}", std::env::consts::EXE_SUFFIX))
}

pub fn build_binaries() -> Result<()> {
    // Windows ikilileri Wine altında koşarken cargo çağrılamaz; önceden derlenmiş olmalı.
    if std::env::var("ULAK_SKIP_BUILD").is_ok() {
        return Ok(());
    }
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let release = bin_dir().ends_with("release");
    let mut cmd = Command::new(cargo);
    cmd.args(["build", "-q", "-p", "divan", "-p", "ulak"]).current_dir(workspace_root());
    if release {
        cmd.arg("--release");
    }
    let st = cmd.status().context("cargo build çalıştırılamadı")?;
    if !st.success() {
        bail!("cargo build başarısız");
    }
    Ok(())
}

pub fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

pub struct Server {
    pub data_dir: PathBuf,
    pub port: u16,
    pub extra_env: Vec<(String, String)>,
    child: Option<Child>,
}

impl Server {
    pub fn new(data_dir: &Path) -> Server {
        Server {
            data_dir: data_dir.to_path_buf(),
            port: free_port(),
            extra_env: vec![],
            child: None,
        }
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn start(&mut self) -> Result<()> {
        let mut cmd = Command::new(exe("divan"));
        cmd.env("DATA_DIR", &self.data_dir)
            .env("BIND_ADDR", format!("127.0.0.1:{}", self.port))
            .env("PUBLIC_URL", self.url())
            .env("CI_API_TOKEN", TOKEN)
            .env("GITLAB_WEBHOOK_TOKEN", "prova-webhook")
            .env("LOG_STDOUT", "false")
            .env("DRY_RUN", "true")
            .env_remove("ADMIN_PASSWORD_HASH")
            .current_dir(&self.data_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit());
        for (k, v) in &self.extra_env {
            cmd.env(k, v);
        }
        std::fs::create_dir_all(&self.data_dir)?;
        self.child = Some(cmd.spawn().context("divan başlatılamadı")?);
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            if let Ok(r) = get(&format!("{}/api/health", self.url())) {
                if r.0 == 200 {
                    return Ok(());
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        bail!("sunucu 20 sn içinde açılmadı")
    }

    pub fn stop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop();
    }
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .max_redirects(0)
        .proxy(None)
        .timeout_global(Some(Duration::from_secs(60)))
        .build()
        .into()
}

pub fn get(url: &str) -> Result<(u16, String)> {
    let mut r = agent().get(url).call()?;
    let mut s = String::new();
    r.body_mut().as_reader().read_to_string(&mut s)?;
    Ok((r.status().as_u16(), s))
}

pub type Headers = Vec<(String, String)>;

pub fn get_with(url: &str, headers: &[(&str, &str)]) -> Result<(u16, Headers, Vec<u8>)> {
    let mut b = agent().get(url);
    for (k, v) in headers {
        b = b.header(*k, *v);
    }
    let mut r = b.call()?;
    let hs = r
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let mut out = Vec::new();
    r.body_mut().as_reader().read_to_end(&mut out)?;
    Ok((r.status().as_u16(), hs, out))
}

pub fn get_bytes(url: &str) -> Result<(u16, Vec<u8>)> {
    let (s, _, b) = get_with(url, &[])?;
    Ok((s, b))
}

pub fn get_json(url: &str) -> Result<serde_json::Value> {
    let (s, body) = get(url)?;
    if s != 200 {
        bail!("{url} → {s}: {body}");
    }
    Ok(serde_json::from_str(&body)?)
}

pub fn post(url: &str, headers: &[(&str, &str)], body: &[u8]) -> Result<(u16, Headers, String)> {
    let mut b = agent().post(url);
    for (k, v) in headers {
        b = b.header(*k, *v);
    }
    let mut r = b.send(body)?;
    let hs = r
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let mut s = String::new();
    r.body_mut().as_reader().read_to_string(&mut s)?;
    Ok((r.status().as_u16(), hs, s))
}

pub fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)? {
        let e = e?;
        let n = e.file_name();
        if n == "target" || n == ".ulak" {
            continue;
        }
        if e.path().is_dir() {
            copy_dir(&e.path(), &dst.join(&n))?;
        } else {
            std::fs::copy(e.path(), dst.join(&n))?;
        }
    }
    Ok(())
}

pub fn sample_dir() -> PathBuf {
    workspace_root().join("testdata/allure/ornek-giris")
}

/// ulak komutu: gerçek CI'daki gibi proje klasöründe, ortam değişkenleriyle.
pub struct Ulak {
    pub exe: PathBuf,
    pub project: PathBuf,
    pub env: Vec<(String, String)>,
}

pub struct Output {
    pub code: i32,
    pub text: String,
}

impl Ulak {
    pub fn new(exe: &Path, project: &Path, server: &str) -> Ulak {
        Ulak {
            exe: exe.to_path_buf(),
            project: project.to_path_buf(),
            env: vec![
                ("ULAK_SERVER".into(), server.into()),
                ("CI_API_TOKEN".into(), TOKEN.into()),
                ("CI_PROJECT_ID".into(), "4242".into()),
                ("CI_PROJECT_PATH".into(), "prova/ornek-test-projesi".into()),
                (
                    "CI_PROJECT_URL".into(),
                    "https://gitlab.sirket.local/prova/ornek-test-projesi".into(),
                ),
                ("CI_COMMIT_REF_NAME".into(), "main".into()),
                ("CI_COMMIT_SHA".into(), "0123456789abcdef0123456789abcdef01234567".into()),
                ("CI_PIPELINE_ID".into(), "777".into()),
                (
                    "CI_PIPELINE_URL".into(),
                    "https://gitlab.sirket.local/prova/ornek-test-projesi/-/pipelines/777".into(),
                ),
                (
                    "CI_JOB_URL".into(),
                    "https://gitlab.sirket.local/prova/ornek-test-projesi/-/jobs/1".into(),
                ),
                ("ULAK_RETRY_BASE_MS".into(), "200".into()),
            ],
        }
    }

    pub fn set(&mut self, k: &str, v: impl Into<String>) -> &mut Self {
        self.env.retain(|(a, _)| a != k);
        self.env.push((k.into(), v.into()));
        self
    }

    pub fn command(&self, args: &[&str]) -> Command {
        let mut c = Command::new(&self.exe);
        c.args(args).current_dir(&self.project).stdin(Stdio::null());
        for (k, v) in &self.env {
            c.env(k, v);
        }
        c
    }

    pub fn run(&self, args: &[&str]) -> Result<Output> {
        let out = self
            .command(args)
            .output()
            .with_context(|| format!("ulak {args:?} çalıştırılamadı"))?;
        let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        Ok(Output {
            code: out.status.code().unwrap_or(-1),
            text,
        })
    }
}

/// Adım adım doğrulama çıktısı.
#[derive(Default)]
pub struct Checks {
    pub failed: Vec<String>,
    pub passed: usize,
}

impl Checks {
    pub fn check(&mut self, id: &str, what: &str, ok: bool, detail: impl AsRef<str>) {
        if ok {
            self.passed += 1;
            println!("  [GEÇTİ] {id:<8} {what}");
        } else {
            println!(
                "  [KALDI] {id:<8} {what}\n           {}",
                detail.as_ref().replace('\n', "\n           ")
            );
            self.failed.push(format!("{id} {what}"));
        }
    }

    pub fn result(self) -> i32 {
        println!("\n{} kontrol geçti, {} kaldı", self.passed, self.failed.len());
        for f in &self.failed {
            println!("  - {f}");
        }
        if self.failed.is_empty() {
            0
        } else {
            1
        }
    }
}
