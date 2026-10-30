//! Prova (SW-10.2): CI akışını yerelde benzetir. Her yeni CI adımı buraya eklenir.
//!
//!   cargo run -p rehearse
//!
//! Sunucu gerçek bir süreç olarak açılır; stajci gerçek ikilidir. Testler yerine `fake-tests`
//! alt komutu Allure örneklerini (testdata) senaryo senaryo yazar.

use anyhow::Result;
use common::allure::{file_kind, parse_container, parse_result, FileKind};
use rehearse::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn now_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
}

/// Sahte test koşusu: stajci'nin Maven'a verdiği argümanları alır, sonuçları sırayla yazar.
fn fake_tests(args: &[String]) -> i32 {
    let mut sample = sample_dir();
    let mut delay = 200u64;
    let mut env_out: Option<PathBuf> = None;
    let mut results: Option<PathBuf> = None;
    let mut oom = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--sample" => {
                sample = PathBuf::from(&args[i + 1]);
                i += 1;
            }
            "--delay-ms" => {
                delay = args[i + 1].parse().unwrap();
                i += 1;
            }
            "--env-out" => {
                env_out = Some(PathBuf::from(&args[i + 1]));
                i += 1;
            }
            "--oom" => oom = true,
            a if a.starts_with("-Dallure.results.directory=") => results = Some(PathBuf::from(&a["-Dallure.results.directory=".len()..])),
            _ => {}
        }
        i += 1;
    }
    if let Some(p) = &env_out {
        let info = serde_json::json!({
            "args": args,
            "maven_opts": std::env::var("MAVEN_OPTS").unwrap_or_default(),
            "display": std::env::var("DISPLAY").unwrap_or_default(),
        });
        std::fs::write(p, info.to_string()).unwrap();
    }
    if oom {
        println!("[INFO] Running RunCucumberTest");
        eprintln!("Exception in thread \"main\" java.lang.OutOfMemoryError: Java heap space");
        return 1;
    }
    let results = results.expect("-Dallure.results.directory yok");
    std::fs::create_dir_all(&results).unwrap();
    // sonuçları başlama zamanına göre sırala, her sonuçtan önce ekleri ve container'ı yaz
    let mut res = Vec::new();
    let mut containers: HashMap<String, Vec<PathBuf>> = HashMap::new();
    let mut container_atts: HashMap<PathBuf, Vec<String>> = HashMap::new();
    for e in std::fs::read_dir(&sample).unwrap().flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        match file_kind(&n) {
            FileKind::Result => res.push(parse_result(&std::fs::read(e.path()).unwrap()).unwrap()),
            FileKind::Container => {
                let c = parse_container(&std::fs::read(e.path()).unwrap()).unwrap();
                for ch in &c.children {
                    containers.entry(ch.clone()).or_default().push(e.path());
                }
                container_atts.insert(
                    e.path(),
                    c.afters
                        .iter()
                        .chain(c.befores.iter())
                        .flat_map(|f| f.attachments.iter().map(|a| a.source.clone()))
                        .collect(),
                );
            }
            _ => {}
        }
    }
    res.sort_by_key(|r| r.start.unwrap_or(0));
    let mut failed = false;
    let log = env_out.as_ref().map(|p| p.with_extension("written"));
    for (idx, r) in res.iter().enumerate() {
        std::thread::sleep(Duration::from_millis(delay));
        let mut files: Vec<String> = r.all_attachments().iter().map(|a| a.source.clone()).collect();
        for cp in containers.get(&r.uuid).cloned().unwrap_or_default() {
            files.extend(container_atts.get(&cp).cloned().unwrap_or_default());
            files.push(cp.file_name().unwrap().to_string_lossy().to_string());
        }
        files.push(format!("{}-result.json", r.uuid));
        for f in files {
            let src = sample.join(&f);
            if src.exists() {
                std::fs::copy(&src, results.join(&f)).unwrap();
            }
        }
        if let Some(l) = &log {
            use std::io::Write;
            let mut fh = std::fs::OpenOptions::new().create(true).append(true).open(l).unwrap();
            writeln!(fh, "{} {}", idx + 1, now_ms()).unwrap();
        }
        failed |= r.status().is_failure();
        println!("[fake-tests] {} → {}", r.name, r.status().as_str());
    }
    if failed {
        1
    } else {
        0
    }
}

fn project_copy(root: &Path, name: &str) -> Result<PathBuf> {
    let p = root.join(name);
    copy_dir(&workspace_root().join("ornek-test-projesi"), &p)?;
    Ok(p)
}

fn total_attempts(run: &serde_json::Value) -> i64 {
    run["senaryolar"]
        .as_array()
        .map(|a| a.iter().map(|s| s["deneme_sayisi"].as_i64().unwrap_or(0)).sum())
        .unwrap_or(0)
}

fn fake_cmd(sample: &Path, delay: u64, env_out: &Path, extra: &str) -> String {
    format!(
        "\"{}\" fake-tests --sample \"{}\" --delay-ms {delay} --env-out \"{}\" {extra}",
        exe("rehearse").display(),
        sample.display(),
        env_out.display()
    )
}

fn template_checks(ck: &mut Checks) -> Result<()> {
    let text = std::fs::read_to_string(workspace_root().join("ci/gitlab/template/.gitlab-ci.yml"))?;
    let y: serde_yaml::Value = serde_yaml::from_str(&text)?;
    let stages: Vec<String> = y["stages"]
        .as_sequence()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect();
    ck.check(
        "SW-1.1",
        "şablonda test → report → fix aşamaları",
        stages == ["test", "report", "fix"],
        format!("{stages:?}"),
    );
    for os in ["linux", "windows"] {
        let tag_ok = ["test", "report", "fix"].iter().all(|k| {
            let job = &y[format!("staj-{k}-{os}").as_str()];
            job.is_mapping()
        });
        ck.check("SW-1.1", &format!("{os} için 3 iş tanımlı"), tag_ok, "");
        let base = &y[format!(".staj-{os}").as_str()];
        ck.check(
            "SW-1.1",
            &format!("{os} işleri '{os}' etiketli runner'a gider"),
            base["tags"][0].as_str() == Some(os),
            "",
        );
        for k in ["report", "fix"] {
            let rules = y[format!("staj-{k}-{os}").as_str()]["rules"].as_sequence().unwrap();
            let last = rules.last().unwrap();
            ck.check(
                "SW-1.1",
                &format!("{k}-{os} test işi düşse de çalışır (when: always)"),
                last["when"].as_str() == Some("always"),
                "",
            );
        }
    }
    let mut only_stajci = true;
    for k in [".staj-test", ".staj-report", ".staj-fix"] {
        for line in y[k]["script"].as_sequence().unwrap() {
            only_stajci &= line.as_str().unwrap_or("").starts_with("stajci ");
        }
    }
    ck.check(
        "SW-1.1",
        "script satırları yalnızca stajci alt komutlarını çağırır",
        only_stajci,
        "",
    );
    let vars = &y["variables"];
    ck.check(
        "SW-1.7",
        "bellek sınırları CI değişkeni (STAJ_MAVEN_XMX, STAJ_TEST_XMX)",
        vars["STAJ_MAVEN_XMX"].is_string() && vars["STAJ_TEST_XMX"].is_string(),
        "",
    );
    ck.check(
        "SW-1.2",
        "Linux'ta Xvfb sabit çözünürlük (STAJ_XVFB, STAJ_SCREEN)",
        vars["STAJ_XVFB"].as_str() == Some("on") && vars["STAJ_SCREEN"].is_string(),
        "",
    );
    ck.check(
        "SW-5.4",
        "rapor linki pipeline environment linkinde",
        y[".staj-report"]["environment"]["url"].as_str() == Some("$STAJ_REPORT_URL"),
        "",
    );
    ck.check(
        "SW-2.6",
        "gönderilemeyen sonuçlar artifact olarak saklanır",
        y[".staj-report"]["artifacts"]["paths"]
            .as_sequence()
            .is_some_and(|p| p.iter().any(|x| x.as_str() == Some(".stajci/queue/"))),
        "",
    );
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("fake-tests") => std::process::exit(fake_tests(&args[1..])),
        Some("fake-opencode") => std::process::exit(fake_opencode()),
        Some("fake-validate") => std::process::exit(
            std::env::var("STAJ_FAKE_VALIDATE")
                .map(|v| if v == "fail" { 1 } else { 0 })
                .unwrap_or(0),
        ),
        _ => {}
    }
    println!("== raporcu prova (SW-10.2) ==");
    build_binaries()?;
    let mut ck = Checks::default();
    let tmp = tempfile::tempdir()?;
    let root = tmp.path();

    println!("\n-- CI şablonu");
    template_checks(&mut ck)?;

    println!("\n-- Sunucu ve stajci dağıtımı");
    let mut server = Server::new(&root.join("data"));
    std::fs::create_dir_all(server.data_dir.join("tools"))?;
    let tools_name = if cfg!(windows) { "stajci-windows.exe" } else { "stajci-linux" };
    std::fs::copy(exe("stajci"), server.data_dir.join("tools").join(tools_name))?;
    server.start()?;
    let os = if cfg!(windows) { "windows" } else { "linux" };
    let (st, bin) = get_bytes(&format!("{}/api/tools/stajci?os={os}", server.url()))?;
    let proj = project_copy(root, "proje")?;
    let stajci_path = proj.join(format!("stajci{}", std::env::consts::EXE_SUFFIX));
    std::fs::write(&stajci_path, &bin)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&stajci_path, std::fs::Permissions::from_mode(0o755))?;
    }
    let mut s = Stajci::new(&stajci_path, &proj, &server.url());
    let ver = s.run(&["--version"])?;
    ck.check(
        "SW-10.9",
        "stajci sunucudan indirilir ve çalışır",
        st == 200 && ver.code == 0,
        format!("{st} {}", ver.text),
    );

    println!("\n-- Ön kontrol");
    let pf = s.run(&["preflight"])?;
    ck.check("SW-1.5", "örnek proje ön kontrolü geçer", pf.code == 0, &pf.text);
    let bad = project_copy(root, "eksik")?;
    let pom = std::fs::read_to_string(bad.join("pom.xml"))?.replace("allure-cucumber7-jvm", "yok");
    std::fs::write(bad.join("pom.xml"), pom)?;
    let sb = Stajci::new(&stajci_path, &bad, &server.url());
    let pf = sb.run(&["preflight"])?;
    ck.check(
        "SW-1.5",
        "adaptör eksikse açık mesajla düşer",
        pf.code == 1 && pf.text.contains("Allure adaptörü eksik"),
        &pf.text,
    );

    println!("\n-- Test işi: start + watch (canlı)");
    let env_out = root.join("env-1.json");
    s.set("STAJ_TEST_COMMAND", fake_cmd(&sample_dir(), 700, &env_out, ""))
        .set("STAJ_FORCE_MAVEN_ARGS", "1")
        .set("STAJ_WATCH_INTERVAL_MS", "300");
    let has_xvfb = cfg!(target_os = "linux") && Path::new("/usr/bin/xvfb-run").exists();
    s.set("STAJ_XVFB", if has_xvfb { "on" } else { "off" });
    // ikinci proje aynı anda koşar (SW-1.2: ekranlar karışmaz)
    let proj2 = project_copy(root, "proje2")?;
    let env_out2 = root.join("env-2.json");
    let mut s2 = Stajci::new(&stajci_path, &proj2, &server.url());
    s2.set("STAJ_TEST_COMMAND", fake_cmd(&sample_dir(), 100, &env_out2, ""))
        .set("STAJ_FORCE_MAVEN_ARGS", "1")
        .set("CI_PROJECT_ID", "4343")
        .set("CI_PROJECT_PATH", "prova/ikinci")
        .set("STAJ_XVFB", if has_xvfb { "on" } else { "off" });
    let mut child2 = s2.command(&["start"]).stdout(std::process::Stdio::null()).spawn()?;
    let mut child = s.command(&["start"]).stdout(std::process::Stdio::piped()).spawn()?;
    // koşum id'si
    let t0 = Instant::now();
    let mut run_id = 0;
    while t0.elapsed() < Duration::from_secs(15) {
        if let Ok(t) = std::fs::read_to_string(proj.join(".stajci/run.json")) {
            run_id = serde_json::from_str::<serde_json::Value>(&t)?["run_id"].as_i64().unwrap_or(0);
            if run_id > 0 {
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let mut progress_seen = String::new();
    let mut visible_at: HashMap<i64, u128> = HashMap::new();
    while child.try_wait()?.is_none() {
        if let Ok(run) = get_json(&format!("{}/api/runs/{run_id}", server.url())) {
            let n = total_attempts(&run);
            for k in 1..=n {
                visible_at.entry(k).or_insert_with(now_ms);
            }
            if run["kosum"]["status"] == "running" && run["kosum"]["total"].as_i64().unwrap_or(0) > 0 {
                progress_seen = format!("{}/{}", run["kosum"]["total"], run["kosum"]["total_expected"]);
            }
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    let code = child.wait()?.code().unwrap_or(-1);
    let _ = child2.wait();
    let mut out = String::new();
    std::io::Read::read_to_string(child.stdout.as_mut().unwrap(), &mut out)?;
    ck.check(
        "SW-3.1",
        "koşum testler bitmeden ilerlemeyle görünür (x/7)",
        progress_seen.ends_with("/7"),
        format!("görülen: {progress_seen:?}"),
    );
    ck.check("SW-1.1", "test işi kalan senaryoda düşer (çıkış 1)", code == 1, &out);
    let written = std::fs::read_to_string(env_out.with_extension("written")).unwrap_or_default();
    let mut worst = 0u128;
    let mut missing = 0;
    for line in written.lines() {
        let mut it = line.split_whitespace();
        let (k, t): (i64, u128) = (it.next().unwrap().parse()?, it.next().unwrap().parse()?);
        match visible_at.get(&k) {
            Some(v) => worst = worst.max(v.saturating_sub(t)),
            None => missing += 1,
        }
    }
    // son sonuçlar test bittikten sonra görünebilir; onları sayma
    ck.check(
        "SW-3.2",
        "senaryo bittikten en geç 5 sn sonra görünür",
        worst <= 5000 && missing <= 2,
        format!("en kötü {worst} ms, görülmeyen {missing}"),
    );
    let info: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&env_out)?)?;
    let args_s = info["args"].to_string();
    ck.check(
        "SW-1.7",
        "Maven ve test JVM'ine bellek sınırı verilir",
        info["maven_opts"].as_str().unwrap_or("").contains("-Xmx512m") && args_s.contains("-Dtest.xmx=768m"),
        info.to_string(),
    );
    ck.check(
        "SW-1.5",
        "features.txt tag'leri testlere geçer",
        args_s.contains("-Dcucumber.filter.tags=(@giris)"),
        &args_s,
    );
    if has_xvfb {
        let info2: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&env_out2).unwrap_or("{}".into()))?;
        let (d1, d2) = (info["display"].as_str().unwrap_or(""), info2["display"].as_str().unwrap_or(""));
        ck.check(
            "SW-1.2",
            "testler Xvfb ekranında koşar; aynı anda iki iş farklı ekran alır",
            !d1.is_empty() && !d2.is_empty() && d1 != d2,
            format!("{d1} / {d2}"),
        );
    } else {
        println!("  [ATLANDI] SW-1.2  xvfb-run yok");
    }

    println!("\n-- Report işi: upload + finish");
    let up = s.run(&["upload"])?;
    let env_file = std::fs::read_to_string(proj.join("stajci.env")).unwrap_or_default();
    ck.check(
        "SW-5.4",
        "rapor linki yükleme çıktısında ve stajci.env'de",
        up.code == 0 && up.text.contains(&format!("Rapor: {}/runs/{run_id}", server.url())) && env_file.contains("STAJ_REPORT_URL="),
        &up.text,
    );
    ck.check(
        "SW-10.7",
        "stajci çıktısında her satırda run_id",
        up.text
            .lines()
            .filter(|l| l.starts_with("[stajci"))
            .all(|l| l.contains(&format!("run_id={run_id}"))),
        &up.text,
    );
    let fin = s.run(&["finish"])?;
    let run = get_json(&format!("{}/api/runs/{run_id}", server.url()))?;
    ck.check(
        "SW-3.5",
        "finish koşumu bitirir",
        fin.code == 0 && run["kosum"]["status"] == "finished",
        &fin.text,
    );
    ck.check(
        "SW-3.3",
        "artımlı + toplu sonuç tutarlı: 7 senaryo, 11 deneme, tekrar yok",
        run["kosum"]["total"] == 7 && total_attempts(&run) == 11 && run["kosum"]["passed"] == 4,
        run["kosum"].to_string(),
    );
    ck.check(
        "SW-4.3",
        "CI bilgisi kaydedildi",
        run["kosum"]["pipeline_id"] == "777" && run["kosum"]["commit_sha"].as_str().is_some_and(|c| c.starts_with("0123")),
        run["kosum"].to_string(),
    );

    println!("\n-- Bellek sınırı aşımı");
    let mut s3 = Stajci::new(&stajci_path, &proj, &server.url());
    s3.set("STAJ_TEST_COMMAND", fake_cmd(&sample_dir(), 0, &root.join("env-oom.json"), "--oom"))
        .set("STAJ_FORCE_MAVEN_ARGS", "1")
        .set("STAJ_XVFB", "off");
    let o = s3.run(&["start"])?;
    ck.check(
        "SW-1.7",
        "OutOfMemoryError → iş açık mesajla düşer",
        o.code == 3 && o.text.contains("BELLEK SINIRI AŞILDI"),
        &o.text,
    );

    println!("\n-- Windows oturum kontrolü");
    if !cfg!(windows) {
        let mut s4 = Stajci::new(&stajci_path, &proj, &server.url());
        s4.set("STAJ_FAKE_WINDOWS_SESSION", "Services");
        let o = s4.run(&["start", "--no-tests"])?;
        ck.check(
            "SW-1.6",
            "runner hizmet oturumundaysa açık mesajla düşer (benzetim)",
            o.code == 4 && o.text.contains("masaüstü oturumunda değil"),
            &o.text,
        );
    }

    println!("\n-- Sunucu koşum sırasında kapanıp açılır (SW-2.6)");
    let proj3 = project_copy(root, "proje3")?;
    let mut s5 = Stajci::new(&stajci_path, &proj3, &server.url());
    s5.set("STAJ_TEST_COMMAND", fake_cmd(&sample_dir(), 500, &root.join("env-3.json"), ""))
        .set("STAJ_FORCE_MAVEN_ARGS", "1")
        .set("STAJ_XVFB", "off")
        .set("STAJ_WATCH_INTERVAL_MS", "200")
        .set("CI_PROJECT_PATH", "prova/kesinti")
        .set("CI_PROJECT_ID", "4444");
    let mut ch = s5.command(&["start"]).stdout(std::process::Stdio::null()).spawn()?;
    std::thread::sleep(Duration::from_millis(2200));
    server.stop();
    std::thread::sleep(Duration::from_millis(1500));
    server.start()?;
    ch.wait()?;
    let up = s5.run(&["upload"])?;
    let fin = s5.run(&["finish"])?;
    let rid: i64 = serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(proj3.join(".stajci/run.json"))?)?["run_id"]
        .as_i64()
        .unwrap();
    let run = get_json(&format!("{}/api/runs/{rid}", server.url()))?;
    ck.check(
        "SW-2.6",
        "kesintiden sonra hiçbir sonuç eksik ya da çift değil",
        up.code == 0 && fin.code == 0 && run["kosum"]["total"] == 7 && total_attempts(&run) == 11,
        format!("{}\n{}\n{}", up.text, fin.text, run["kosum"]),
    );

    println!("\n-- Sunucu yokken yükleme kuyruğa alınır, --resume ile gönderilir");
    let proj4 = project_copy(root, "proje4")?;
    copy_dir(&sample_dir(), &proj4.join("target/allure-results"))?;
    let mut s6 = Stajci::new(&stajci_path, &proj4, &server.url());
    s6.set("STAJ_RETRY_MAX", "1")
        .set("STAJ_RETRY_BASE_MS", "100")
        .set("CI_PROJECT_PATH", "prova/kuyruk")
        .set("CI_PROJECT_ID", "4545");
    server.stop();
    let up = s6.run(&["upload"])?;
    let queued = std::fs::read_dir(proj4.join(".stajci/queue")).map(|r| r.count()).unwrap_or(0);
    ck.check(
        "SW-2.6",
        "ulaşılamayınca kuyruğa yazılır",
        up.code == 1 && queued == 2 && up.text.contains("--resume"),
        &up.text,
    );
    server.start()?;
    let rs = s6.run(&["upload", "--resume"])?;
    let queued = std::fs::read_dir(proj4.join(".stajci/queue")).map(|r| r.count()).unwrap_or(0);
    let list = get_json(&format!("{}/api/runs", server.url()))?;
    let found = list["kosumlar"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["project_path"] == "prova/kuyruk" && r["total"] == 7);
    ck.check(
        "SW-2.6",
        "--resume kuyruğu gönderir",
        rs.code == 0 && queued == 0 && found,
        &rs.text,
    );

    extra_steps(&mut ck, &server, &stajci_path, root)?;

    server.stop();
    std::process::exit(ck.result());
}

/// Sahte OpenCode: STAJ_FAKE_MODE'a göre test kodunu değiştirir.
fn fake_opencode() -> i32 {
    let mode = std::env::var("STAJ_FAKE_MODE").unwrap_or_else(|_| "ok".into());
    let prompt = std::fs::read_to_string(std::env::var("STAJ_FIX_PROMPT_FILE").unwrap_or_default()).unwrap_or_default();
    let steps = Path::new("src/test/java/com/ornek/test/GirisAdimlari.java");
    let mut code = std::fs::read_to_string(steps).unwrap();
    match mode.as_str() {
        "nochange" => {}
        "assert" => {
            code = code.replace(
                "assertEquals(beklenen, mesaj.getText());",
                "assertTrue(mesaj.getText().contains(beklenen));",
            )
        }
        _ => code = code.replace("robot.clickOn(\"#sifremiUnuttum\");", "robot.clickOn(\"#girisButonu\");"),
    }
    std::fs::write(steps, code).unwrap();
    if mode == "outside" {
        let pom = std::fs::read_to_string("pom.xml").unwrap();
        std::fs::write(
            "pom.xml",
            pom.replace("<rerun.count>1</rerun.count>", "<rerun.count>5</rerun.count>"),
        )
        .unwrap();
    }
    println!(
        "istem {} karakter, ekran görüntüsü: {}",
        prompt.len(),
        prompt.contains("Ekran görüntüsü")
    );
    println!("STAJ_OZET: #sifremiUnuttum seçicisi olmayan bir düğümü arıyordu");
    println!("STAJ_TRIAGE: {{\"category\": \"test_kodu\", \"reason\": \"Seçici yanlış\", \"confidence\": 0.9}}");
    0
}

/// Sahte GitLab MR ucu: her POST'u kaydeder, iid döner.
fn fake_gitlab() -> (u16, std::sync::Arc<std::sync::Mutex<Vec<String>>>) {
    use std::io::{BufRead, BufReader, Read, Write};
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let got = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let g = got.clone();
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            let mut r = BufReader::new(s.try_clone().unwrap());
            let mut len = 0usize;
            let mut line = String::new();
            loop {
                line.clear();
                if r.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                    break;
                }
                if let Some(v) = line.to_lowercase().strip_prefix("content-length:") {
                    len = v.trim().parse().unwrap_or(0);
                }
            }
            let mut body = vec![0; len];
            let _ = r.read_exact(&mut body);
            let n = {
                let mut v = g.lock().unwrap();
                v.push(String::from_utf8_lossy(&body).to_string());
                v.len()
            };
            let resp =
                format!("{{\"iid\": {n}, \"web_url\": \"https://gitlab.sirket.local/prova/ornek-test-projesi/-/merge_requests/{n}\"}}");
            let mut w = s;
            let _ = write!(
                w,
                "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp}",
                resp.len()
            );
        }
    });
    (port, got)
}

fn sh(dir: &Path, args: &[&str]) -> Result<String> {
    let o = std::process::Command::new("git").args(args).current_dir(dir).output()?;
    anyhow::ensure!(o.status.success(), "git {:?}: {}", args, String::from_utf8_lossy(&o.stderr));
    Ok(String::from_utf8_lossy(&o.stdout).to_string())
}

/// Self-healing (SW-7.2, 9.4, 7.3, 4.4, 5.4): sahte OpenCode + sahte GitLab + yerel git deposu.
fn extra_steps(ck: &mut Checks, server: &Server, stajci: &Path, root: &Path) -> Result<()> {
    println!("\n-- Fix işi: stajci fix (sahte OpenCode ve GitLab)");
    if std::process::Command::new("git").arg("--version").output().is_err() {
        println!("  [ATLANDI] SW-7.2  git bulunamadı");
        return Ok(());
    }
    let list = get_json(&format!("{}/api/runs", server.url()))?;
    let run_id = list["kosumlar"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["project_path"] == "prova/ornek-test-projesi")
        .map(|r| r["id"].as_i64().unwrap())
        .min()
        .unwrap();
    let proj = project_copy(root, "fix-proje")?;
    let bare = root.join("uzak.git");
    sh(root, &["init", "-q", "--bare", bare.to_str().unwrap()])?;
    sh(&proj, &["init", "-q", "-b", "main"])?;
    std::fs::write(proj.join(".gitignore"), "target/\n.stajci/\nstajci\nstajci.exe\nstajci.env\n")?;
    sh(&proj, &["add", "-A"])?;
    sh(&proj, &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "ilk"])?;
    let (gl_port, mrs) = fake_gitlab();
    let mut s = Stajci::new(stajci, &proj, &server.url());
    s.set("STAJ_RUN_ID", run_id.to_string())
        .set(
            "STAJ_OPENCODE_CMD",
            format!("\"{}\" fake-opencode {{istem}}", exe("rehearse").display()),
        )
        .set("STAJ_FIX_VALIDATE_CMD", format!("\"{}\" fake-validate", exe("rehearse").display()))
        .set("STAJ_XVFB", "off")
        .set("STAJ_GITLAB_API", format!("http://127.0.0.1:{gl_port}/api/v4"))
        .set("STAJ_FIX_TOKEN", "sahte-fix-token")
        .set("STAJ_PUSH_REMOTE", bare.to_str().unwrap())
        .set("STAJ_REPORT_URL", format!("{}/runs/{run_id}", server.url()));
    let o = s.run(&["fix"])?;
    let branches = sh(&bare, &["branch", "--list"]).unwrap_or_default();
    let mr = mrs.lock().unwrap().first().cloned().unwrap_or_default();
    let mrv: serde_json::Value = serde_json::from_str(&mr).unwrap_or_default();
    ck.check(
        "SW-7.2",
        "test kodu düzeltilir, auto-fix branch'i push edilir, MR açılır",
        o.code == 0 && branches.contains("auto-fix/run-") && o.text.contains("MR açıldı: !1"),
        format!("{}\n{branches}", o.text),
    );
    ck.check(
        "SW-7.2",
        "doğrulama geçince MR Draft değil",
        !mrv["title"].as_str().unwrap_or("Draft").starts_with("Draft"),
        &mr,
    );
    ck.check(
        "SW-5.4",
        "rapor linki MR açıklamasında",
        mrv["description"].as_str().unwrap_or("").contains(&format!("/runs/{run_id}")),
        &mr,
    );
    let diff = sh(&bare, &["log", "-p", "-1", "--all", "--", "src/test"]).unwrap_or_default();
    ck.check(
        "SW-7.2",
        "değişiklik yalnızca src/test altında",
        diff.contains("#girisButonu") && !diff.contains("pom.xml"),
        &diff,
    );
    let (_, html) = get(&format!("{}/runs/{run_id}", server.url()))?;
    ck.check(
        "SW-4.4",
        "raporda düzeltme kutusu ve OpenCode !MR rozeti",
        html.contains("OpenCode düzeltmeleri") && html.contains("OpenCode !1"),
        "",
    );
    ck.check(
        "SW-7.3",
        "fix-result rapora yazıldı (OpenCode triage önerisi dahil)",
        html.contains("Test kodu hatası"),
        "",
    );

    s.set("STAJ_FAKE_VALIDATE", "fail");
    let o = s.run(&["fix"])?;
    let mr = mrs.lock().unwrap().get(1).cloned().unwrap_or_default();
    let mrv: serde_json::Value = serde_json::from_str(&mr).unwrap_or_default();
    ck.check(
        "SW-7.2",
        "doğrulama geçmezse Draft MR",
        mrv["title"].as_str().unwrap_or("").starts_with("Draft:"),
        format!("{}\n{mr}", o.text),
    );

    s.set("STAJ_FAKE_VALIDATE", "ok").set("STAJ_FAKE_MODE", "outside");
    let before = mrs.lock().unwrap().len();
    let o = s.run(&["fix"])?;
    let clean = sh(&proj, &["status", "--porcelain"]).unwrap_or_default();
    ck.check(
        "SW-9.4",
        "izinli yolların dışındaki değişiklik reddedilir",
        o.text.contains("REDDEDİLDİ") && mrs.lock().unwrap().len() == before && clean.trim().is_empty(),
        format!("{}\n{clean}", o.text),
    );

    s.set("STAJ_FAKE_MODE", "assert");
    s.run(&["fix"])?;
    let mr = mrs.lock().unwrap().last().cloned().unwrap_or_default();
    ck.check(
        "SW-9.4",
        "assertion değiştiren düzeltmede MR'a uyarı eklenir",
        mr.contains("assertion"),
        &mr,
    );
    let (_, html) = get(&format!("{}/runs/{run_id}", server.url()))?;
    ck.check(
        "SW-9.4",
        "reddedilen deneme raporda görünür",
        html.contains("Reddedildi (koruma)"),
        "",
    );
    Ok(())
}
