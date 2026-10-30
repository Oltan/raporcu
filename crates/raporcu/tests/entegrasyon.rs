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
    /// bu alan yoksa Jira 400 döner (zorunlu özel alan benzetimi)
    require_field: Option<String>,
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
        if let Some(f) = g.require_field.clone() {
            if v["fields"].get(&f).is_none() {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "errorMessages": [], "errors": { f: "Faz is required." } })),
                );
            }
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
    async fn issuetypes() -> Json<Value> {
        Json(json!({ "values": [{ "id": "10004", "name": "Bug" }, { "id": "10001", "name": "Task" }] }))
    }
    async fn fields(Path((_p, t)): Path<(String, String)>) -> Json<Value> {
        assert_eq!(t, "10004");
        Json(json!({ "values": [
            { "fieldId": "summary", "name": "Summary", "required": true, "schema": { "type": "string" } },
            { "fieldId": "customfield_10200", "name": "Faz", "required": true, "hasDefaultValue": false,
              "schema": { "type": "option", "custom": "com.atlassian.jira.plugin.system.customfieldtypes:select" },
              "allowedValues": [{ "id": "10300", "value": "Yazılım" }, { "id": "10301", "value": "Sistem" }] },
            { "fieldId": "priority", "name": "Priority", "required": true, "hasDefaultValue": true,
              "schema": { "type": "priority" }, "allowedValues": [{ "id": "2", "name": "Critical" }, { "id": "3", "name": "Major" }] },
            { "fieldId": "components", "name": "Component/s", "required": false,
              "schema": { "type": "array", "items": "component" }, "allowedValues": [{ "id": "500", "name": "UI" }, { "id": "501", "name": "Giriş" }] }
        ] }))
    }
    spawn_fake(
        Router::new()
            .route("/rest/api/2/issue/createmeta/{p}/issuetypes", get(issuetypes))
            .route("/rest/api/2/issue/createmeta/{p}/issuetypes/{t}", get(fields))
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
    let (proj, tok, body) = got.lock().unwrap()[0].clone();
    assert_eq!(proj, "55");
    assert_eq!(tok, "glpat-gizli-token");
    assert_eq!(body["ref"], "main");
    let vars = body["variables"].as_array().unwrap();
    assert!(vars.iter().any(|v| v["key"] == "STAJ_FIX_ONLY" && v["value"] == "true"));
    assert!(vars
        .iter()
        .any(|v| v["key"] == "STAJ_RUN_ID" && v["value"] == id.to_string().as_str()));
    assert!(
        !vars.iter().any(|v| v["key"] == "STAJ_FIX_SCENARIO_ID"),
        "koşum düğmesi: tüm adaylar"
    );
    // senaryo düğmesi: yalnızca o senaryo
    let sid = scenario_id(&run_json(&app, id).await, "Yanlış şifrede özel mesaj");
    let (_, page) = app.get_admin(&format!("/scenarios/{sid}")).await;
    assert!(page.contains(&format!("/admin/scenarios/{sid}/fix")));
    let r = post_form(&app, &format!("/admin/scenarios/{sid}/fix"), &app.admin_cookie(), "").await;
    assert_eq!(r.status(), StatusCode::SEE_OTHER);
    assert_eq!(r.headers()["location"], format!("/scenarios/{sid}?notice=fix").as_str());
    let g = got.lock().unwrap();
    let vars = g[1].2["variables"].as_array().unwrap();
    assert!(vars
        .iter()
        .any(|v| v["key"] == "STAJ_FIX_SCENARIO_ID" && v["value"] == sid.to_string().as_str()));
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

// --- Jira şablonu, zorunlu alanlar ve bug formu; DOORS ayarları -----------------------------

#[tokio::test]
async fn test_sw_6_1_required_fields_listed_and_bug_form_sends_them() {
    let log: JL = Default::default();
    log.lock().unwrap().require_field = Some("customfield_10200".into());
    let url = fake_jira(log.clone()).await;
    let app = app_with(jira_cfg(url, raporcu::config::JiraMode::Off));
    let cookie = app.admin_cookie();
    let id = upload_sample(&app, Q).await;
    let sid = scenario_id(&run_json(&app, id).await, "Yanlış şifrede özel mesaj");

    // zorunlu alan şablonda yokken: Jira'nın hatası alan adıyla ve ne yapılacağıyla gösterilir
    let r = post_form(&app, &format!("/admin/scenarios/{sid}/jira"), &cookie, "").await;
    let body = body_string(r).await;
    assert!(body.contains("customfield_10200: Faz is required."), "{body}");
    assert!(body.contains("Ek alanlar"));
    // formdan gönderilip Jira reddederse: form, girilen değerlerle ve hatayla yeniden gelir
    let r = post_form(
        &app,
        &format!("/admin/scenarios/{sid}/jira"),
        &cookie,
        "ozet=Benim+ozetim&aciklama=x&ek=",
    )
    .await;
    assert_eq!(r.status(), StatusCode::BAD_GATEWAY);
    let body = body_string(r).await;
    assert!(
        body.contains(r#"value="Benim ozetim""#) && body.contains("Faz is required."),
        "{body}"
    );

    // Ayarlar: Jira alanları zorunlu/izinli değerleriyle listelenir
    let (st, page) = app.get_admin("/admin/ayarlar?alanlar=1").await;
    assert_eq!(st, StatusCode::OK);
    assert!(page.contains("customfield_10200") && page.contains("zorunlu") && page.contains("10300"));
    assert!(page.contains("Yazılım"));

    // şablon: özet biçimi + ek alanlar (öncelik, yer tutuculu metin)
    let form = format!(
        "ozet={}&aciklama={}&alanlar={}&doors_label=on",
        urlencoding::encode("[E2E] {{senaryo}} ({{branch}})"),
        urlencoding::encode("Rapor: {{rapor}}\n{noformat}{{hata}}{noformat}"),
        urlencoding::encode(r#"{"priority": {"name": "Major"}, "components": [{"name": "UI"}]}"#)
    );
    let r = post_form(&app, "/admin/ayarlar/jira", &cookie, &form).await;
    assert_eq!(r.status(), StatusCode::SEE_OTHER);
    // bozuk JSON kaydedilmez, girilen metin kaybolmaz
    let r = post_form(&app, "/admin/ayarlar/jira", &cookie, "ozet=x&aciklama=y&alanlar=%7Bbozuk").await;
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    assert!(body_string(r).await.contains("{bozuk"));

    // form: şablondan dolu; zorunlu "Faz" seçim listesi olarak, öncelik varsayılanlı ama şablonda olduğu için gösterilir
    let (st, f) = app.get_admin(&format!("/admin/scenarios/{sid}/jira")).await;
    assert_eq!(st, StatusCode::OK);
    assert!(f.contains("[E2E] Yanlış şifrede özel mesaj (main)"), "{f}");
    assert!(f.contains(r#"name="alan.customfield_10200""#) && f.contains(r#"<option value="10301""#));
    assert!(f.contains(r#"name="alan.components""#), "şablondaki çoklu alan formda");
    assert!(f.contains(&format!("/scenarios/{sid}")), "rapor linki açıklamada");

    let send = format!(
        "ozet={}&aciklama=a&doors_label=on&ek=&alan.customfield_10200=10301&tur.customfield_10200={}&alan.components=500&alan.components=501&tur.components={}&alan.priority=Critical&tur.priority={}",
        urlencoding::encode("Elle özet"),
        urlencoding::encode("option||1"),
        urlencoding::encode("array|component|1"),
        urlencoding::encode("priority||0"),
    );
    let r = post_form(&app, &format!("/admin/scenarios/{sid}/jira"), &cookie, &send).await;
    assert_eq!(r.status(), StatusCode::SEE_OTHER);
    let g = log.lock().unwrap();
    let f = &g.created[0]["fields"];
    assert_eq!(f["summary"], "Elle özet");
    assert_eq!(f["customfield_10200"], json!({ "id": "10301" }));
    assert_eq!(f["components"], json!([{ "id": "500" }, { "id": "501" }]));
    assert_eq!(f["priority"], json!({ "name": "Critical" }), "formdaki değer şablonu ezer");
    assert_eq!(f["labels"], json!(["DOORS-1003"]));
    assert_eq!(f["project"]["key"], "STAJ");
}

#[tokio::test]
async fn test_sw_6_3_auto_bug_uses_template_fields() {
    let log: JL = Default::default();
    log.lock().unwrap().require_field = Some("customfield_10200".into());
    let url = fake_jira(log.clone()).await;
    let app = app_with(jira_cfg(url, raporcu::config::JiraMode::On));
    let form = format!(
        "ozet={}&aciklama=x&doors_label=on&alanlar={}",
        urlencoding::encode("{{proje}}: {{senaryo}}"),
        urlencoding::encode(r#"{"customfield_10200": {"value": "Yazılım"}, "labels": ["otomasyon"]}"#)
    );
    let r = post_form(&app, "/admin/ayarlar/jira", &app.admin_cookie(), &form).await;
    assert_eq!(r.status(), StatusCode::SEE_OTHER);
    app.ci_post(&format!("/api/ci/runs?{Q}"), "application/zip", zip_without_jira_tag())
        .await;
    let mut created = vec![];
    for _ in 0..50 {
        settle().await;
        created = log.lock().unwrap().created.clone();
        if !created.is_empty() {
            break;
        }
    }
    assert!(!created.is_empty(), "otomatik bug açılmadı");
    let f = &created[0]["fields"];
    assert_eq!(f["customfield_10200"]["value"], "Yazılım");
    assert!(f["summary"].as_str().unwrap().starts_with("ekip/javafx: "), "{f}");
    let labels = f["labels"].as_array().unwrap();
    assert!(labels.contains(&json!("otomasyon")) && labels.iter().any(|l| l.as_str().unwrap().starts_with("DOORS-")));
}

#[tokio::test]
async fn test_sw_6_4_doors_settings_used_in_dxl_csv_and_links() {
    let app = app_with(|c| c.doors_dxl = true);
    let cookie = app.admin_cookie();
    let id = upload_sample(&app, Q).await;
    let form = "modul=%2FProje%2FSRS&id_oznitelik=Object+Identifier&sonuc_oznitelik=Test+Result&tarih_oznitelik=&jira_oznitelik=Jira&rapor_oznitelik=&gecti=Passed&kaldi=Failed&atlandi=Skipped&kosulmadi=Not+Run&link_sablonu=doors%3A%2F%2Fdoors.sirket.local%3A36677%2F%3Furn%3Dx-%7B%7Bno%7D%7D";
    let r = post_form(&app, "/admin/ayarlar/doors", &cookie, form).await;
    assert_eq!(r.status(), StatusCode::SEE_OTHER);
    let (_, dxl) = app.get(&format!("/runs/{id}/doors.dxl")).await;
    assert!(dxl.contains("string modulYolu = \"/Proje/SRS\""));
    assert!(dxl.contains("m = edit(modulYolu, false)"));
    assert!(dxl.contains("string idAttr = \"Object Identifier\""));
    assert!(dxl.contains("string dateAttr = \"\""), "boş öznitelik yazılmaz");
    assert!(dxl.contains("\"Failed\"") && !dxl.contains("\"Kaldı\""));
    let (_, csv) = app.get(&format!("/runs/{id}/doors.csv")).await;
    assert!(csv.contains("DOORS-1001,Geçerli bilgilerle giriş,Giriş ekranı,Passed"));
    let sid = scenario_id(&run_json(&app, id).await, "Yanlış şifrede özel mesaj");
    let (_, page) = app.get(&format!("/scenarios/{sid}")).await;
    assert!(
        page.contains(r#"href="doors://doors.sirket.local:36677/?urn=x-1003""#),
        "DOORS rozeti link"
    );
    // ayarlar yeniden açılışta veritabanından okunur
    let c = app.state.db.read_blocking(raporcu::ayarlar::load).unwrap();
    assert_eq!(c.doors.modul, "/Proje/SRS");
    // boş kimlik özniteliği kaydedilmez
    let r = post_form(&app, "/admin/ayarlar/doors", &cookie, "id_oznitelik=").await;
    assert!(r.status().is_client_error() || r.status().is_server_error());
}

#[tokio::test]
async fn test_sw_6_4_doors_csv_settings_kept_when_dxl_off() {
    let app = app();
    let cookie = app.admin_cookie();
    let (_, page) = app.get_admin("/admin/ayarlar").await;
    assert!(page.contains("DOORS_DXL=true") && page.contains(r#"type="hidden" name="id_oznitelik""#));
    // DXL alanları gizliyken kaydetmek onları silmez; CSV sonuç değerleri değişir
    let form = "modul=&id_oznitelik=Absolute+Number&sonuc_oznitelik=Test+Sonucu&tarih_oznitelik=Test+Tarihi&jira_oznitelik=Jira&rapor_oznitelik=Test+Raporu&gecti=Passed&kaldi=Failed&atlandi=Skipped&kosulmadi=Not+Run&link_sablonu=";
    let r = post_form(&app, "/admin/ayarlar/doors", &cookie, form).await;
    assert_eq!(r.status(), StatusCode::SEE_OTHER);
    let id = upload_sample(&app, Q).await;
    let (_, csv) = app.get(&format!("/runs/{id}/doors.csv")).await;
    assert!(csv.contains("DOORS-1003,Yanlış şifrede özel mesaj,Giriş ekranı,Failed"), "{csv}");
    assert_eq!(app.state.ayarlar.get().doors.id_oznitelik, "Absolute Number");
}
