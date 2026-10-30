//! `stajci start`: koşumu açar (SW-3.1), izleyiciyi ayrı süreç olarak başlatır (SW-3.2) ve
//! testleri koşar.
//!
//! - Linux: uygulama Xvfb ekranında gerçekten açılır; `xvfb-run -a` her işe boş bir ekran
//!   numarası verir (SW-1.2). Headless/Monocle yok.
//! - Windows: runner oturum açmış kullanıcının masaüstünde çalışmalıdır (SW-1.6); hizmet
//!   oturumu tespit edilirse iş açık mesajla düşer.
//! - Bellek: `MAVEN_OPTS=-Xmx…`, `-Dtest.xmx=…` (surefire argLine), tek fork; OutOfMemoryError
//!   görülürse açık mesajla düşer (SW-1.7).

use crate::ctx::{self, env, env_or, say, Ctx, SavedRun};
use crate::gherkin;
use anyhow::{Context, Result};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const EXIT_OOM: i32 = 3;
pub const EXIT_SESSION: i32 = 4;

pub fn tag_expression(c: &Ctx) -> Result<Option<String>> {
    if let Some(t) = env("STAJ_TAGS") {
        gherkin::TagExpr::parse(&t)?;
        return Ok(Some(t));
    }
    Ok(gherkin::read_features_txt(&c.project_dir.join("features.txt"))?
        .map(|(lines, _)| lines.iter().map(|l| format!("({l})")).collect::<Vec<_>>().join(" or ")))
}

/// SW-1.6: Windows'ta runner'ın masaüstü oturumunda olduğunu doğrular.
pub fn windows_session_problem() -> Option<String> {
    if !cfg!(windows) && env("STAJ_FAKE_WINDOWS_SESSION").is_none() {
        return None;
    }
    if env("STAJ_ALLOW_SERVICE_SESSION").as_deref() == Some("true") {
        return None;
    }
    let session = env("SESSIONNAME").or_else(|| env("STAJ_FAKE_WINDOWS_SESSION"));
    let user = env("USERNAME").unwrap_or_default();
    if user.ends_with('$') || session.is_none() || session.as_deref() == Some("Services") {
        Some(format!(
            "GitLab runner masaüstü oturumunda değil (kullanıcı: {user}, oturum: {}). TestFX fare/klavye olaylarını \
             gönderemez. Runner'ı Windows hizmeti olarak değil, otomatik oturum açan kullanıcının masaüstünde \
             başlatın (docs/GITLAB_CI_KURULUMU.md, SW-1.6).",
            session.unwrap_or_else(|| "yok".into())
        ))
    } else {
        None
    }
}

pub struct TestCommand {
    pub program: String,
    pub args: Vec<String>,
    pub envs: Vec<(String, String)>,
}

pub fn build_test_command(c: &Ctx, tags: Option<&str>) -> TestCommand {
    let mut parts = ctx::split_command(&env_or("STAJ_TEST_COMMAND", "mvn -B test"));
    let mut program = parts.remove(0);
    let is_maven = program == "mvn" || program.ends_with("mvnw") || program.ends_with("mvnw.cmd");
    if cfg!(windows) && program == "mvn" {
        program = "mvn.cmd".into();
    }
    let maven_xmx = env_or("STAJ_MAVEN_XMX", "512m");
    let test_xmx = env_or("STAJ_TEST_XMX", "768m");
    if is_maven || env("STAJ_FORCE_MAVEN_ARGS").is_some() {
        parts.push(format!("-Dtest.xmx={test_xmx}"));
        parts.push(format!("-Dallure.results.directory={}", c.results_dir.display()));
        // canlı olay eklentisi (CanliOlayEklentisi) bu dosyaya yazar, stajci watch okur
        parts.push(format!("-Dstajci.canli.dosya={}", c.live_events_file().display()));
        if let Some(t) = tags {
            parts.push(format!("-Dcucumber.filter.tags={t}"));
        }
    }
    if let Some(extra) = env("STAJ_EXTRA_ARGS") {
        parts.extend(ctx::split_command(&extra));
    }
    let mut maven_opts = env("MAVEN_OPTS").unwrap_or_default();
    if !maven_opts.contains("-Xmx") {
        maven_opts = format!("{maven_opts} -Xmx{maven_xmx} -XX:+ExitOnOutOfMemoryError")
            .trim()
            .to_string();
    }
    if !maven_opts.contains("stdout.encoding") {
        maven_opts.push_str(" -Dstdout.encoding=UTF-8 -Dstderr.encoding=UTF-8");
    }
    let envs = vec![
        ("MAVEN_OPTS".to_string(), maven_opts),
        ("STAJ_VIDEO_FPS".to_string(), env_or("STAJ_VIDEO_FPS", "5")),
    ];
    wrap_display(TestCommand {
        program,
        args: parts,
        envs,
    })
}

/// SW-1.2: Linux'ta komutu Xvfb ekranında çalıştırır (`xvfb-run -a`: her işe boş ekran numarası).
pub fn wrap_display(mut cmd: TestCommand) -> TestCommand {
    let xvfb = env_or("STAJ_XVFB", "auto");
    let use_xvfb = cfg!(target_os = "linux") && (xvfb == "on" || (xvfb == "auto" && env("DISPLAY").is_none()));
    if !use_xvfb {
        return cmd;
    }
    let screen = env_or("STAJ_SCREEN", "1920x1080");
    let mut args = vec![
        "-a".to_string(),
        "-s".to_string(),
        format!("-screen 0 {screen}x24 -nolisten tcp"),
        cmd.program,
    ];
    args.append(&mut cmd.args);
    cmd.envs.push(("GDK_BACKEND".into(), "x11".into()));
    TestCommand {
        program: env_or("STAJ_XVFB_RUN", "xvfb-run"),
        args,
        envs: cmd.envs,
    }
}

fn is_oom_line(l: &str) -> bool {
    l.contains("java.lang.OutOfMemoryError")
        || l.contains("Terminating due to java.lang.OutOfMemoryError")
        || l.contains("Cannot allocate memory")
}

/// Komutu koşar; çıktıyı satır satır basar, bellek hatasını yakalar. (çıkış kodu, oom)
pub fn run_streaming(cmd: &TestCommand, cwd: &std::path::Path) -> Result<(i32, bool)> {
    let mut command = Command::new(&cmd.program);
    command
        .args(&cmd.args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in &cmd.envs {
        command.env(k, v);
    }
    let mut child = command
        .spawn()
        .with_context(|| format!("test komutu başlatılamadı: {}", cmd.program))?;
    let oom = Arc::new(AtomicBool::new(false));
    let mut handles = Vec::new();
    for stream in [
        child.stdout.take().map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
        child.stderr.take().map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
    ]
    .into_iter()
    .flatten()
    {
        let oom = oom.clone();
        handles.push(std::thread::spawn(move || {
            for line in BufReader::new(stream).split(b'\n').map_while(Result::ok) {
                let l = String::from_utf8_lossy(&line);
                if is_oom_line(&l) {
                    oom.store(true, Ordering::Relaxed);
                }
                println!("{}", l.trim_end_matches('\r'));
            }
        }));
    }
    let status = child.wait()?;
    for h in handles {
        let _ = h.join();
    }
    Ok((status.code().unwrap_or(1), oom.load(Ordering::Relaxed)))
}

fn spawn_watcher(c: &Ctx, run_id: i64) -> Option<std::process::Child> {
    if env("STAJ_WATCH").as_deref() == Some("off") {
        return None;
    }
    let exe = std::env::current_exe().ok()?;
    let log = std::fs::File::create(c.state_dir.join("watch.log")).ok()?;
    let log2 = log.try_clone().ok()?;
    match Command::new(exe)
        .arg("watch")
        .arg("--run-id")
        .arg(run_id.to_string())
        .current_dir(&c.project_dir)
        .stdin(Stdio::null())
        .stdout(log)
        .stderr(log2)
        .spawn()
    {
        Ok(ch) => Some(ch),
        Err(e) => {
            say(format!("izleyici başlatılamadı ({e}); sonuçlar iş sonunda toplu gönderilecek"));
            None
        }
    }
}

pub fn start(c: &Ctx, run_tests: bool) -> Result<i32> {
    if let Some(p) = windows_session_problem() {
        say(format!("HATA: {p}"));
        return Ok(EXIT_SESSION);
    }
    std::fs::create_dir_all(&c.state_dir)?;
    let _ = std::fs::remove_file(c.stop_file());
    let _ = std::fs::remove_file(c.state_dir.join("sent.txt"));
    // önceki koşumun sonuçları karışmasın
    if run_tests && c.results_dir.exists() && env("STAJ_KEEP_RESULTS").is_none() {
        let _ = std::fs::remove_dir_all(&c.results_dir);
        let _ = std::fs::remove_file(c.live_events_file());
    }
    let tags = tag_expression(c)?;
    let expr = tags.as_deref().map(gherkin::TagExpr::parse).transpose()?;
    let planned = gherkin::plan(&c.project_dir, expr.as_ref())?;
    let ci = ctx::ci_info(&c.project_dir);
    let mut body = serde_json::to_value(&ci)?;
    body["total"] = (planned.len() as i64).into();
    body["expected"] = planned
        .iter()
        .map(|p| serde_json::json!({ "name": p.name, "feature": p.feature }))
        .collect::<Vec<_>>()
        .into();
    let client = c.client();
    let reply = client.with_retry("koşum açma", || client.post_json("/api/ci/runs/start", &body));
    let saved = match reply.and_then(|r| crate::client::ensure_ok(r, "koşum açma")) {
        Ok(r) => {
            let v = r.json()?;
            let saved = SavedRun {
                run_id: v["run_id"].as_i64().unwrap_or(0),
                report_url: v["report_url"].as_str().unwrap_or("").to_string(),
            };
            c.save_run(&saved)?;
            ctx::set_run_id(saved.run_id);
            say(format!(
                "koşum açıldı: {} senaryo bekleniyor. Canlı durum: {}/live",
                planned.len(),
                saved.report_url
            ));
            Some(saved)
        }
        Err(e) => {
            if e.to_string().contains("uyumsuz") {
                say(format!("HATA: {e}"));
                return Ok(2);
            }
            say(format!(
                "UYARI: sunucuya ulaşılamadı ({e}); testler yine koşacak, sonuçlar sonra gönderilecek (SW-2.6)"
            ));
            None
        }
    };
    if !run_tests {
        return Ok(0);
    }
    let mut watcher = saved.as_ref().and_then(|s| spawn_watcher(c, s.run_id));
    let cmd = build_test_command(c, tags.as_deref());
    say(format!("testler başlıyor: {} {}", cmd.program, cmd.args.join(" ")));
    let started = Instant::now();
    let result = run_streaming(&cmd, &c.project_dir);
    // izleyiciye bitişi bildir, son dosyaları göndermesi için bekle
    let _ = std::fs::write(c.stop_file(), "bitti");
    if let Some(w) = watcher.as_mut() {
        let deadline = Instant::now() + Duration::from_secs(env("STAJ_WATCH_DRAIN_SECS").and_then(|v| v.parse().ok()).unwrap_or(30));
        loop {
            match w.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(200)),
                _ => {
                    let _ = w.kill();
                    break;
                }
            }
        }
    }
    let (code, oom) = result?;
    say(format!("testler bitti: çıkış kodu {code}, süre {} sn", started.elapsed().as_secs()));
    if oom {
        say(format!(
            "HATA: BELLEK SINIRI AŞILDI (java.lang.OutOfMemoryError). Test JVM sınırı STAJ_TEST_XMX={}, Maven sınırı STAJ_MAVEN_XMX={}. \
             Gerekirse CI değişkeniyle artırın (SW-1.7).",
            env_or("STAJ_TEST_XMX", "768m"),
            env_or("STAJ_MAVEN_XMX", "512m")
        ));
        return Ok(EXIT_OOM);
    }
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_1_7_memory_limits_in_test_command() {
        let c = Ctx {
            server: String::new(),
            token: String::new(),
            project_dir: ".".into(),
            state_dir: ".stajci".into(),
            results_dir: "r".into(),
        };
        let cmd = build_test_command(&c, Some("@smoke"));
        let all = format!("{} {}", cmd.program, cmd.args.join(" "));
        assert!(all.contains("-Dtest.xmx="));
        assert!(all.contains("-Dcucumber.filter.tags=@smoke"));
        let mo = &cmd.envs.iter().find(|(k, _)| k == "MAVEN_OPTS").unwrap().1;
        assert!(mo.contains("-Xmx"));
    }

    #[test]
    fn test_sw_1_7_oom_detected() {
        assert!(is_oom_line(
            "Exception in thread \"main\" java.lang.OutOfMemoryError: Java heap space"
        ));
        assert!(!is_oom_line("Tests run: 3"));
    }
}
