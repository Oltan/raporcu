//! Faz 6–8: webhook, e-posta, Jira, triage, self-healing uçları (sahte Jira / GitLab ile)
mod util;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use util::*;

const Q: &str = "project_id=55&project_path=ekip%2Fjavafx&branch=main&pipeline_id=9001";

fn pipeline_hook(status: &str, source: &str) -> Value {
    json!({
        "object_kind": "pipeline",
        "object_attributes": { "id": 9001, "ref": "main", "status": status, "source": source, "url": "https://gitlab.sirket/ekip/javafx/-/pipelines/9001" },
        "project": { "id": 55, "path_with_namespace": "ekip/javafx", "web_url": "https://gitlab.sirket/ekip/javafx" }
    })
}

async fn hook(app: &TestApp, token: &str, v: Value) -> StatusCode {
    app.send(
        axum::http::Request::post("/api/gitlab/webhook")
            .header("x-gitlab-token", token)
            .header("content-type", "application/json")
            .body(axum::body::Body::from(v.to_string()))
            .unwrap(),
    )
    .await
    .status()
}

#[tokio::test]
async fn test_sw_9_3_webhook_wrong_token_rejected() {
    let app = app();
    assert_eq!(
        hook(&app, "yanlis", pipeline_hook("running", "push")).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(hook(&app, "", pipeline_hook("running", "push")).await, StatusCode::UNAUTHORIZED);
    assert_eq!(
        hook(&app, "test-webhook-token", pipeline_hook("running", "push")).await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn test_sw_3_6_pipeline_status_on_dashboard() {
    let app = app();
    hook(&app, "test-webhook-token", pipeline_hook("pending", "schedule")).await;
    let (_, d) = app.get("/").await;
    assert!(d.contains("Pipeline'lar") && d.contains("#9001") && d.contains("kuyrukta"));
    hook(&app, "test-webhook-token", pipeline_hook("running", "schedule")).await;
    let (_, d) = app.get("/").await;
    assert!(d.contains("çalışıyor"));
    assert!(log_lines("webhook alındı").iter().any(|l| l["durum"] == "running"));
}

#[tokio::test]
async fn test_sw_5_3_nightly_summary_email_once_per_pipeline_dry_run() {
    let app = app_with(|c| c.mail_to = vec!["ekip@sirket.local".into()]);
    upload_sample(&app, Q).await;
    for _ in 0..3 {
        hook(&app, "test-webhook-token", pipeline_hook("failed", "schedule")).await;
    }
    let outbox: Vec<_> = std::fs::read_dir(app.state.paths.outbox_dir()).unwrap().collect();
    assert_eq!(outbox.len(), 1, "pipeline başına tek e-posta");
    let mail = std::fs::read_to_string(outbox[0].as_ref().unwrap().path()).unwrap();
    assert!(mail.contains("Geçen: 4") && mail.contains("Kalan: 2"));
    assert!(mail.contains("http://raporcu.test/runs/1"));
    assert!(mail.contains("Açılan MR'lar"));
    // push pipeline'ı e-posta göndermez
    let n: i64 = app
        .state
        .db
        .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM emails", [], |r| r.get(0))?))
        .await
        .unwrap();
    assert_eq!(n, 1);
}

#[tokio::test]
async fn test_sw_5_3_smtp_send_when_not_dry_run() {
    // sahte SMTP sunucusu
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let got = Arc::new(Mutex::new(String::new()));
    let g2 = got.clone();
    std::thread::spawn(move || {
        use std::io::{BufRead, BufReader, Write};
        let (s, _) = l.accept().unwrap();
        let mut w = s.try_clone().unwrap();
        let mut r = BufReader::new(s);
        w.write_all(b"220 hazir\r\n").unwrap();
        let mut data = false;
        loop {
            let mut line = String::new();
            if r.read_line(&mut line).unwrap() == 0 {
                break;
            }
            g2.lock().unwrap().push_str(&line);
            if data {
                if line == ".\r\n" {
                    data = false;
                    w.write_all(b"250 tamam\r\n").unwrap();
                }
                continue;
            }
            if line.starts_with("DATA") {
                data = true;
                w.write_all(b"354 devam\r\n").unwrap();
            } else if line.starts_with("QUIT") {
                break;
            } else {
                w.write_all(b"250 tamam\r\n").unwrap();
            }
        }
    });
    let app = app_with(move |c| {
        c.dry_run = false;
        c.smtp_host = Some("127.0.0.1".into());
        c.smtp_port = port;
        c.mail_to = vec!["ekip@sirket.local".into()];
    });
    upload_sample(&app, Q).await;
    hook(&app, "test-webhook-token", pipeline_hook("success", "schedule")).await;
    std::thread::sleep(std::time::Duration::from_millis(200));
    let s = got.lock().unwrap().clone();
    assert!(s.contains("RCPT TO:<ekip@sirket.local>"));
    assert!(s.contains("Geçen: 4"));
}

// --- Sahte Jira -------------------------------------------------------------------------------

#[derive(Default)]
struct JiraLog {
    created: Vec<Value>,
    attachments: Vec<String>,
    searches: Vec<String>,
    reject_labels: bool,
    search_hit: Option<String>,
    auth: Vec<String>,
}

type JL = Arc<Mutex<JiraLog>>;

async fn fake_jira(log: JL) -> String {
    async fn create(State(l): State<JL>, headers: HeaderMap, Json(v): Json<Value>) -> (StatusCode, Json<Value>) {
        let mut g = l.lock().unwrap();
        g.auth.push(
            headers
                .get("authorization")
                .map(|h| h.to_str().unwrap().to_string())
                .unwrap_or_default(),
        );
        if g.reject_labels && v["fields"].get("labels").is_some() {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "errors": { "labels": "Field 'labels' cannot be set." } })),
            );
        }
        g.created.push(v);
        let key = format!("BUG-{}", g.created.len());
        (StatusCode::CREATED, Json(json!({ "key": key })))
    }
    async fn attach(State(l): State<JL>, Path(k): Path<String>) -> Json<Value> {
        l.lock().unwrap().attachments.push(k);
        Json(json!([]))
    }
    async fn search(State(l): State<JL>, axum::extract::RawQuery(q): axum::extract::RawQuery) -> Json<Value> {
        let mut g = l.lock().unwrap();
        g.searches.push(urlencoding::decode(&q.unwrap_or_default()).unwrap().into_owned());
        match &g.search_hit {
            Some(k) => Json(json!({ "issues": [{ "key": k }] })),
            None => Json(json!({ "issues": [] })),
        }
    }
    spawn_fake(
        Router::new()
            .route("/rest/api/2/issue", post(create))
            .route("/rest/api/2/issue/{key}/attachments", post(attach))
            .route("/rest/api/2/search", get(search))
            .with_state(log),
    )
    .await
}

fn jira_cfg(url: String, mode: raporcu::config::JiraMode) -> impl FnOnce(&mut raporcu::Config) {
    move |c| {
        c.jira_url = Some(url);
        c.jira_token = Some(raporcu::config::Secret::new("jira-gizli-pat"));
        c.jira_project = Some("STAJ".into());
        c.jira_mode = mode;
    }
}

/// Yalnızca "Yanlış şifrede özel mesaj" kalan, Jira tag'siz örnek.
fn zip_without_jira_tag() -> Vec<u8> {
    let tmp = tempfile::tempdir().unwrap();
    for e in std::fs::read_dir(sample_dir()).unwrap() {
        let p = e.unwrap().path();
        let n = p.file_name().unwrap().to_string_lossy().to_string();
        let mut data = std::fs::read(&p).unwrap();
        if n.ends_with("-result.json") {
            let mut v: Value = serde_json::from_slice(&data).unwrap();
            v["labels"] = Value::Array(
                v["labels"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|l| l["value"] != "JIRA-STAJ-42")
                    .cloned()
                    .collect(),
            );
            data = v.to_string().into_bytes();
        }
        std::fs::write(tmp.path().join(&n), data).unwrap();
    }
    zip_dir(tmp.path())
}

async fn upload_zip(app: &TestApp, zip: Vec<u8>) -> i64 {
    let (st, b) = app.ci_post(&format!("/api/ci/runs?{Q}"), "application/zip", zip).await;
    assert_eq!(st, StatusCode::OK, "{b}");
    serde_json::from_str::<Value>(&b).unwrap()["run_id"].as_i64().unwrap()
}

#[tokio::test]
async fn test_sw_6_1_bug_has_doors_in_label_and_description_and_screenshot() {
    let log: JL = Default::default();
    let url = fake_jira(log.clone()).await;
    let app = app_with(jira_cfg(url, raporcu::config::JiraMode::Off));
    let id = upload_sample(&app, Q).await;
    let sid = scenario_id(&run_json(&app, id).await, "Yanlış şifrede özel mesaj");
    let r = post_form(&app, &format!("/admin/scenarios/{sid}/jira"), &app.admin_cookie(), "").await;
    assert_eq!(r.status(), StatusCode::SEE_OTHER);
    let g = log.lock().unwrap();
    let f = &g.created[0]["fields"];
    assert_eq!(f["labels"][0], "DOORS-1003");
    assert!(f["description"].as_str().unwrap().contains("DOORS-1003"));
    assert_eq!(g.attachments, vec!["BUG-1"], "ekran görüntüsü issue'ya eklendi");
    assert!(g.auth[0].starts_with("Bearer "));
}

#[tokio::test]
async fn test_sw_6_1_label_rejected_bug_still_created() {
    let log: JL = Default::default();
    log.lock().unwrap().reject_labels = true;
    let url = fake_jira(log.clone()).await;
    let app = app_with(jira_cfg(url, raporcu::config::JiraMode::Off));
    let id = upload_sample(&app, Q).await;
    let sid = scenario_id(&run_json(&app, id).await, "Yanlış şifrede özel mesaj");
    post_form(&app, &format!("/admin/scenarios/{sid}/jira"), &app.admin_cookie(), "").await;
    let g = log.lock().unwrap();
    assert_eq!(g.created.len(), 1);
    assert!(g.created[0]["fields"].get("labels").is_none());
    assert!(g.created[0]["fields"]["description"].as_str().unwrap().contains("DOORS-1003"));
}

#[tokio::test]
async fn test_sw_6_2_auto_match_after_upload() {
    let log: JL = Default::default();
    log.lock().unwrap().search_hit = Some("STAJ-77".into());
    let url = fake_jira(log.clone()).await;
    let app = app_with(jira_cfg(url, raporcu::config::JiraMode::Observe));
    // 1) tag ile bağlanır
    let id = upload_sample(&app, Q).await;
    settle().await;
    let sid = scenario_id(&run_json(&app, id).await, "Yanlış şifrede özel mesaj");
    let (_, page) = app.get(&format!("/scenarios/{sid}")).await;
    assert!(page.contains("STAJ-42") && page.contains(">tag<"));
    // 2) tag yoksa Jira'da açık bug aranır ve bağlanır, ekran görüntüsü eklenir
    let id2 = upload_zip(&app, zip_without_jira_tag()).await;
    settle().await;
    let sid2 = scenario_id(&run_json(&app, id2).await, "Yanlış şifrede özel mesaj");
    let (_, page) = app.get(&format!("/scenarios/{sid2}")).await;
    assert!(page.contains("STAJ-77"), "açık bug'a bağlandı");
    let g = log.lock().unwrap();
    assert!(g.searches[0].contains("DOORS-1003"));
    assert_eq!(g.attachments, vec!["STAJ-77"]);
    assert!(g.created.is_empty());
    drop(g);
    assert!(log_lines("Jira bug'ı eşleşti").iter().any(|l| l["issue"] == "STAJ-77"));
}

#[tokio::test]
async fn test_sw_6_3_observe_mode_records_without_writing_to_jira() {
    let log: JL = Default::default();
    let url = fake_jira(log.clone()).await;
    let app = app_with(jira_cfg(url, raporcu::config::JiraMode::Observe));
    let id = upload_zip(&app, zip_without_jira_tag()).await;
    settle().await;
    assert!(log.lock().unwrap().created.is_empty(), "gözlem modunda Jira'ya yazılmaz");
    let n: i64 = app
        .state
        .db
        .read(move |c| {
            Ok(c.query_row(
                "SELECT COUNT(*) FROM jira_actions WHERE run_id = ?1 AND action = 'acilacakti'",
                [id],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(n, 1, "kayıt tutulur");
    let (_, admin) = app.get_admin("/admin").await;
    assert!(admin.contains("acilacakti"));
    assert!(!log_lines("açılacaktı").is_empty());
}

#[tokio::test]
async fn test_sw_6_3_live_mode_cap_dedup_and_retry_required() {
    let log: JL = Default::default();
    let url = fake_jira(log.clone()).await;
    let app = app_with(|c| {
        jira_cfg(url, raporcu::config::JiraMode::On)(c);
        c.jira_max_per_run = 1;
    });
    // aynı hata iki farklı senaryoda + biri retry'sız
    let tmp = tempfile::tempdir().unwrap();
    for e in std::fs::read_dir(sample_dir()).unwrap() {
        let p = e.unwrap().path();
        let n = p.file_name().unwrap().to_string_lossy().to_string();
        let mut data = std::fs::read(&p).unwrap();
        if n.ends_with("-result.json") {
            let mut v: Value = serde_json::from_slice(&data).unwrap();
            v["labels"] = Value::Array(
                v["labels"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|l| l["value"] != "JIRA-STAJ-42")
                    .cloned()
                    .collect(),
            );
            if v["name"] == "Yavaş açılan karşılama mesajı" {
                // rerun'da da kalsın → iki deneme, ürün hatası
                v["status"] = "failed".into();
                v["statusDetails"]["message"] = "expected: <A> but was: <B>".into();
            }
            data = v.to_string().into_bytes();
        }
        std::fs::write(tmp.path().join(&n), data).unwrap();
    }
    upload_zip(&app, zip_dir(tmp.path())).await;
    settle().await;
    assert_eq!(log.lock().unwrap().created.len(), 1, "koşum başına üst sınır aşılmaz");
    // ikinci koşum: aynı hata → yeni bug açılmaz, öncekine bağlanır
    upload_zip(&app, zip_dir(tmp.path())).await;
    settle().await;
    let created = log.lock().unwrap().created.len();
    assert!(created <= 2);
    let links: i64 = app
        .state
        .db
        .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM bug_links WHERE source = 'eslesme'", [], |r| r.get(0))?))
        .await
        .unwrap();
    assert!(links >= 1, "aynı hata için tek bug");
}

#[tokio::test]
async fn test_sw_4_5_triage_rule_opencode_admin_precedence_and_history() {
    let app = app();
    let id = upload_sample(&app, Q).await;
    let run = run_json(&app, id).await;
    let failed = scenario_id(&run, "Yanlış şifrede özel mesaj");
    let broken = scenario_id(&run, "Şifremi unuttum bağlantısı");
    let (_, html) = app.get(&format!("/runs/{id}")).await;
    assert!(html.contains("Ürün hatası") && html.contains("Test kodu hatası") && html.contains("Kural"));
    // OpenCode düşük güven: öneri kalır, kural geçerli
    let key = run["senaryolar"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["senaryo"]["id"] == broken)
        .unwrap()["senaryo"]["scenario_key"]
        .as_str()
        .unwrap()
        .to_string();
    app.ci_json(
        &format!("/api/ci/runs/{id}/fix-result"),
        json!({ "scenario_key": key, "outcome": "no_change", "triage": { "category": "ortam", "reason": "ekran yok", "confidence": 0.4 } }),
    )
    .await;
    let (_, sp) = app.get(&format!("/scenarios/{broken}")).await;
    assert!(sp.contains("kaynak: Kural"));
    // OpenCode yüksek güven: geçerli olur
    app.ci_json(&format!("/api/ci/runs/{id}/fix-result"), json!({ "scenario_key": key, "outcome": "no_change", "triage": { "category": "ortam", "reason": "X11 bağlantısı yok", "confidence": 0.9 } })).await;
    let (_, sp) = app.get(&format!("/scenarios/{broken}")).await;
    assert!(sp.contains("Ortam sorunu") && sp.contains("kaynak: OpenCode"));
    // admin değiştirir: geçerli olur, eskiler geçmişte kalır
    let cookie = app.admin_cookie();
    let r = post_form(
        &app,
        &format!("/admin/scenarios/{failed}/triage"),
        &cookie,
        "category=test_kodu&reason=Beklenen+metin+eski",
    )
    .await;
    assert_eq!(r.status(), StatusCode::SEE_OTHER);
    let (_, sp) = app.get(&format!("/scenarios/{failed}")).await;
    assert!(sp.contains("kaynak: Admin") && sp.contains("Karar geçmişi") && sp.contains("Ürün hatası"));
    // giriş yoksa değiştirilemez
    let r = post_form(&app, &format!("/admin/scenarios/{failed}/triage"), "", "category=urun&reason=x").await;
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    assert!(log_lines("triage kararı").iter().any(|l| l["kaynak"] == "admin"));
}

#[tokio::test]
async fn test_sw_7_1_fix_candidates_targets() {
    let app = app();
    let id = upload_sample(&app, Q).await;
    let (st, v) = app.ci_json(&format!("/api/ci/runs/{id}/fix-candidates"), json!(null)).await;
    let _ = st;
    // GET ucu
    let r = app
        .send(
            axum::http::Request::get(format!("/api/ci/runs/{id}/fix-candidates"))
                .header("authorization", format!("Bearer {CI_TOKEN}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(r.status(), StatusCode::OK);
    let list: Value = serde_json::from_str(&body_string(r).await).unwrap();
    let _ = v;
    let t = |n: &str| {
        list.as_array().unwrap().iter().find(|c| c["name"] == n).unwrap()["target"]
            .as_str()
            .unwrap()
            .to_string()
    };
    assert_eq!(t("Şifremi unuttum bağlantısı"), "opencode");
    assert_eq!(t("Yanlış şifrede özel mesaj"), "atla", "@sample-fail atlanır");
    let c = list
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "Şifremi unuttum bağlantısı")
        .unwrap();
    assert!(c["screenshot_url"].as_str().unwrap().starts_with("http://raporcu.test/files/"));
    assert_eq!(c["feature_file"], "features/giris.feature");
    // sahne ağacı fix isteğine metin olarak gider
    assert!(c["texts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["name"] == "Sahne ağacı (TestFX)" && t["content"].as_str().unwrap().contains("#girisButonu")));
}

#[tokio::test]
async fn test_sw_7_3_and_4_4_fix_result_box_and_badge_in_report() {
    let app = app();
    let id = upload_sample(&app, Q).await;
    let run = run_json(&app, id).await;
    let key = run["senaryolar"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["senaryo"]["name"] == "Şifremi unuttum bağlantısı")
        .unwrap()["senaryo"]["scenario_key"]
        .as_str()
        .unwrap()
        .to_string();
    let (st, _) = app
        .ci_json(
            &format!("/api/ci/runs/{id}/fix-result"),
            json!({ "scenario_key": key, "outcome": "mr_opened", "mr_iid": 17, "mr_url": "https://gitlab.sirket/ekip/javafx/-/merge_requests/17",
                    "branch": "auto-fix/run-1", "validation_passed": true, "summary": "Seçici #sifremiUnuttum yerine #girisButonu", "warnings": ["Assertion değişti"] }),
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    let (_, html) = app.get(&format!("/runs/{id}")).await;
    assert!(html.contains("OpenCode düzeltmeleri"));
    assert!(html.contains("OpenCode !17"));
    assert!(html.contains("merge_requests/17"));
    assert!(html.contains("Assertion değişti"));
    assert!(log_lines("OpenCode denemesi bitti").iter().any(|l| l["mr"] == 17));
}

#[tokio::test]
async fn test_sw_7_5_healing_panel_counts_with_mr_webhook() {
    let app = app();
    let id = upload_sample(&app, Q).await;
    let run = run_json(&app, id).await;
    let key = run["senaryolar"].as_array().unwrap()[0]["senaryo"]["scenario_key"]
        .as_str()
        .unwrap()
        .to_string();
    for (iid, outcome) in [(1, "mr_opened"), (2, "draft_mr"), (3, "mr_opened")] {
        app.ci_json(
            &format!("/api/ci/runs/{id}/fix-result"),
            json!({ "scenario_key": key, "outcome": outcome, "mr_iid": iid }),
        )
        .await;
    }
    app.ci_json(
        &format!("/api/ci/runs/{id}/fix-result"),
        json!({ "scenario_key": key, "outcome": "no_change" }),
    )
    .await;
    let mr = |iid: i64, state: &str| json!({ "object_kind": "merge_request", "object_attributes": { "iid": iid, "state": state }, "project": { "id": 55, "path_with_namespace": "ekip/javafx" } });
    hook(&app, "test-webhook-token", mr(1, "merged")).await;
    hook(&app, "test-webhook-token", mr(2, "closed")).await;
    let days = app
        .state
        .db
        .read(|c| raporcu::web::pages::healing_days(c, chrono::Utc::now().date_naive()))
        .await
        .unwrap();
    assert_eq!(days.len(), 30, "son 30 gece");
    let today = days.last().unwrap();
    assert_eq!((today.attempts, today.mrs, today.merged, today.rejected), (4, 3, 1, 1));
    let (st, html) = app.get("/healing").await;
    assert_eq!(st, StatusCode::OK);
    assert!(html.contains("Birleştirilen"));
}

#[tokio::test]
async fn test_sw_7_7_previous_successful_fix_given_as_example() {
    let app = app();
    let id = upload_sample(&app, Q).await;
    let run = run_json(&app, id).await;
    let key = run["senaryolar"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["senaryo"]["name"] == "Şifremi unuttum bağlantısı")
        .unwrap()["senaryo"]["scenario_key"]
        .as_str()
        .unwrap()
        .to_string();
    app.ci_json(&format!("/api/ci/runs/{id}/fix-result"), json!({ "scenario_key": key, "outcome": "mr_opened", "mr_iid": 5, "diff": "-robot.clickOn(\"#sifremiUnuttum\")\n+robot.clickOn(\"#sifreSifirla\")" })).await;
    hook(&app, "test-webhook-token", json!({ "object_kind": "merge_request", "object_attributes": { "iid": 5, "state": "merged" }, "project": { "id": 55, "path_with_namespace": "ekip/javafx" } })).await;
    // yeni koşumda benzer senaryo
    let id2 = upload_sample(&app, Q).await;
    let r = app
        .send(
            axum::http::Request::get(format!("/api/ci/runs/{id2}/fix-candidates"))
                .header("authorization", format!("Bearer {CI_TOKEN}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await;
    let list: Value = serde_json::from_str(&body_string(r).await).unwrap();
    let c = list.as_array().unwrap().iter().find(|c| c["target"] == "opencode").unwrap();
    assert_eq!(c["examples"].as_array().unwrap().len(), 1);
    assert!(c["examples"][0]["diff"].as_str().unwrap().contains("#sifreSifirla"));
}

#[tokio::test]
async fn test_sw_7_4_fix_button_triggers_only_fix_job_via_gitlab_api() {
    let got: Arc<Mutex<Vec<(String, String, Value)>>> = Default::default();
    async fn trigger(
        State(g): State<Arc<Mutex<Vec<(String, String, Value)>>>>,
        Path(p): Path<String>,
        headers: HeaderMap,
        Json(v): Json<Value>,
    ) -> Json<Value> {
        g.lock()
            .unwrap()
            .push((p, headers["private-token"].to_str().unwrap().to_string(), v));
        Json(json!({ "id": 4242, "web_url": "https://gitlab/p/-/pipelines/4242" }))
    }
    let url = spawn_fake(
        Router::new()
            .route("/api/v4/projects/{p}/pipeline", post(trigger))
            .with_state(got.clone()),
    )
    .await;
    let app = app_with(move |c| {
        c.gitlab_url = Some(url);
        c.gitlab_token = Some(raporcu::config::Secret::new("glpat-gizli-token"));
    });
    let id = upload_sample(&app, Q).await;
    // giriş yoksa düğme çalışmaz
    let r = post_form(&app, &format!("/admin/runs/{id}/fix"), "", "").await;
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    let r = post_form(&app, &format!("/admin/runs/{id}/fix"), &app.admin_cookie(), "").await;
    assert_eq!(r.status(), StatusCode::SEE_OTHER);
    let g = got.lock().unwrap();
    let (proj, tok, body) = &g[0];
    assert_eq!(proj, "55");
    assert_eq!(tok, "glpat-gizli-token");
    assert_eq!(body["ref"], "main");
    let vars = body["variables"].as_array().unwrap();
    assert!(vars.iter().any(|v| v["key"] == "STAJ_FIX_ONLY" && v["value"] == "true"));
    assert!(vars
        .iter()
        .any(|v| v["key"] == "STAJ_RUN_ID" && v["value"] == id.to_string().as_str()));
}

#[tokio::test]
async fn test_sw_9_2_gitlab_token_not_in_error_messages_or_logs() {
    // GitLab hata döner ve yanıtında token'ı yansıtır
    async fn bad(headers: HeaderMap) -> (StatusCode, String) {
        (
            StatusCode::UNAUTHORIZED,
            format!("401 Unauthorized token={}", headers["private-token"].to_str().unwrap()),
        )
    }
    let url = spawn_fake(Router::new().route("/api/v4/projects/{p}/pipeline", post(bad))).await;
    let app = app_with(move |c| {
        c.gitlab_url = Some(url);
        c.gitlab_token = Some(raporcu::config::Secret::new("glpat-COK-GIZLI-9"));
    });
    let id = upload_sample(&app, Q).await;
    let r = post_form(&app, &format!("/admin/runs/{id}/fix"), &app.admin_cookie(), "").await;
    assert_eq!(r.status(), StatusCode::BAD_GATEWAY);
    let body = body_string(r).await;
    assert!(body.contains("401"));
    assert!(!body.contains("glpat-COK-GIZLI-9"), "hata mesajında token görünmez");
    let all = raporcu::logging::test_capture().lock().unwrap().join("\n");
    assert!(!all.contains("glpat-COK-GIZLI-9"), "logda token görünmez");
    assert!(!all.contains("jira-gizli-pat"));
    assert!(!all.contains("test-ci-token"));
}
