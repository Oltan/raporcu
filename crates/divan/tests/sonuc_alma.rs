//! Faz 2: SW-2.1, 2.2, 2.3, 4.6, 3.3, 4.9 — gerçek Allure örnekleriyle (SW-2.7)
mod util;

use axum::http::StatusCode;
use serde_json::Value;
use util::*;

const Q: &str = "project_id=101&project_path=ekip%2Fjavafx&branch=main&commit=abc123def456&pipeline_id=9001&pipeline_url=https%3A%2F%2Fgitlab%2Fp%2F9001";

async fn upload(app: &TestApp, query: &str, zip: Vec<u8>) -> (StatusCode, Value) {
    let (st, body) = app.ci_post(&format!("/api/ci/runs?{query}"), "application/zip", zip).await;
    (st, serde_json::from_str(&body).unwrap_or(Value::String(body)))
}

async fn run_json(app: &TestApp, id: i64) -> Value {
    let (st, body) = app.get(&format!("/api/runs/{id}")).await;
    assert_eq!(st, StatusCode::OK, "{body}");
    serde_json::from_str(&body).unwrap()
}

fn scen<'a>(run: &'a Value, name: &str) -> &'a Value {
    run["senaryolar"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["senaryo"]["name"] == name)
        .unwrap_or_else(|| panic!("{name} yok"))
}

#[tokio::test]
async fn test_sw_2_1_upload_real_sample_creates_run() {
    let app = app();
    let (st, sum) = upload(&app, Q, zip_dir(&sample_dir())).await;
    assert_eq!(st, StatusCode::OK, "{sum}");
    assert_eq!(sum["accepted_results"], 11);
    let id = sum["run_id"].as_i64().unwrap();
    assert!(sum["report_url"].as_str().unwrap().ends_with(&format!("/runs/{id}")));

    // listede görünür
    let (_, list) = app.get("/api/runs").await;
    assert!(list.contains(&format!("\"id\":{id}")));

    let run = run_json(&app, id).await;
    let k = &run["kosum"];
    assert_eq!(k["status"], "finished");
    assert_eq!(k["total"], 7, "her senaryo bir kez sayılır (SW-4.9)");
    assert_eq!(k["passed"], 4);
    assert_eq!(k["failed"], 1);
    assert_eq!(k["broken"], 1);
    assert_eq!(k["unknown"], 1);
    assert_eq!(k["branch"], "main");
    assert_eq!(k["project_path"], "ekip/javafx");
    assert_eq!(k["pipeline_id"], "9001");

    let f = scen(&run, "Yanlış şifrede özel mesaj");
    assert_eq!(f["senaryo"]["doors_ids"][0], "DOORS-1003");
    assert_eq!(f["senaryo"]["jira_keys"][0], "STAJ-42");
    assert_eq!(f["senaryo"]["feature_file"], "features/giris.feature");

    // dosyalar DATA_DIR altında
    let files = app.state.paths.run_files_dir(id);
    assert!(std::fs::read_dir(&files).unwrap().count() >= 10);
    let log = log_lines("yükleme alındı");
    assert!(log
        .iter()
        .any(|l| l["run_id"] == id && l["boyut"].as_u64().unwrap() > 0 && l.get("sure_ms").is_some()));
    assert!(log_lines("koşum açıldı").iter().any(|l| l["run_id"] == id));
    assert!(log_lines("koşum bitti").iter().any(|l| l["run_id"] == id));
}

#[tokio::test]
async fn test_sw_2_2_after_hook_screenshot_linked_to_failed_scenario_only() {
    let app = app();
    let (_, sum) = upload(&app, Q, zip_dir(&sample_dir())).await;
    let run = run_json(&app, sum["run_id"].as_i64().unwrap()).await;
    for s in run["senaryolar"].as_array().unwrap() {
        let status = s["senaryo"]["status"].as_str().unwrap();
        let shots: Vec<&Value> = s["ekler"].as_array().unwrap().iter().filter(|a| a["mime"] == "image/png").collect();
        if status == "passed" {
            assert!(
                shots.is_empty(),
                "başarılı senaryoda ekran görüntüsü olmamalı: {}",
                s["senaryo"]["name"]
            );
            assert!(!s["ekler"].as_array().unwrap().iter().any(|a| a["origin"] == "after"));
        }
        if status == "failed" || status == "broken" {
            assert_eq!(shots.len(), 1, "{}", s["senaryo"]["name"]);
            assert_eq!(shots[0]["origin"], "after");
            // ek dosyası sunuluyor
            let (st, _) = app.get(&format!("/files/{}", shots[0]["id"])).await;
            assert_eq!(st, StatusCode::OK);
        }
    }
}

#[tokio::test]
async fn test_sw_2_2_container_before_result_still_links() {
    // Artımlı gönderimde container sonuç dosyasından önce gelebilir.
    let app = app();
    let (_, created) = app
        .ci_json(
            "/api/ci/runs/start",
            serde_json::json!({ "project_path": "ekip/sira", "branch": "main" }),
        )
        .await;
    let id = created["run_id"].as_i64().unwrap();
    let dir = sample_dir();
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    // önce container'lar, sonra ekler, en son sonuçlar
    names.sort_by_key(|n| {
        if n.ends_with("-container.json") {
            0
        } else if n.contains("-attachment") {
            1
        } else {
            2
        }
    });
    for n in &names {
        let (st, _) = app
            .ci_post(
                &format!("/api/ci/runs/{id}/results?name={n}"),
                "application/octet-stream",
                std::fs::read(dir.join(n)).unwrap(),
            )
            .await;
        assert_eq!(st, StatusCode::OK, "{n}");
    }
    let run = run_json(&app, id).await;
    let f = scen(&run, "Yanlış şifrede özel mesaj");
    assert!(f["ekler"].as_array().unwrap().iter().any(|a| a["mime"] == "image/png"));
}

#[tokio::test]
async fn test_sw_2_3_upload_without_results_returns_422() {
    let app = app();
    let zip = zip_dir_filtered(&sample_dir(), |n| !n.ends_with("-result.json"));
    let (st, body) = upload(&app, Q, zip).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body["hata"].as_str().unwrap().contains("Allure eklentisi"));
    // koşum oluşmadı
    let (_, list) = app.get("/api/runs").await;
    assert!(list.contains("\"kosumlar\":[]"));
    assert!(log_lines("yükleme reddedildi").iter().any(|l| l["durum"] == 422));
}

#[tokio::test]
async fn test_sw_2_1_zip_with_dotdot_path_rejected() {
    let app = app();
    let res = std::fs::read(sample_dir().join("0864bd9d-71e8-4096-adf4-fc4ee4c71985-result.json"))
        .ok()
        .unwrap_or_else(|| b"{\"uuid\":\"x\",\"name\":\"x\"}".to_vec());
    let zip = zip_entries(&[("a-result.json", &res), ("../../kotu-attachment.txt", b"x")]);
    let (st, body) = upload(&app, Q, zip).await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "{body}");
    assert!(body["hata"].as_str().unwrap().contains("güvensiz yol"));
    assert!(!app.dir.path().join("kotu-attachment.txt").exists());
}

#[tokio::test]
async fn test_sw_2_1_upload_size_limit() {
    let app = app_with(|c| c.max_upload_bytes = 10_000);
    let (st, _) = upload(&app, Q, zip_dir(&sample_dir())).await;
    assert_eq!(st, StatusCode::PAYLOAD_TOO_LARGE);
    // tmp klasöründe artık dosya kalmaz
    assert_eq!(std::fs::read_dir(app.state.paths.tmp_dir()).unwrap().count(), 0);
}

#[tokio::test]
async fn test_sw_2_1_not_a_zip_rejected() {
    let app = app();
    let (st, _) = upload(&app, Q, b"bu bir zip degil".to_vec()).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_sw_4_6_project_created_from_gitlab_info_and_histories_separate() {
    let app = app();
    let (_, a) = upload(&app, "project_id=1&project_path=ekip%2Fa&branch=main", zip_dir(&sample_dir())).await;
    let (_, b) = upload(&app, "project_id=2&project_path=ekip%2Fb&branch=main", zip_dir(&sample_dir())).await;
    // aynı proje id'si, yol değişse de aynı proje
    let (_, a2) = upload(
        &app,
        "project_id=1&project_path=ekip%2Fa-yeni-ad&branch=develop",
        zip_dir(&sample_dir()),
    )
    .await;
    let ra = run_json(&app, a["run_id"].as_i64().unwrap()).await;
    let rb = run_json(&app, b["run_id"].as_i64().unwrap()).await;
    let ra2 = run_json(&app, a2["run_id"].as_i64().unwrap()).await;
    assert_ne!(ra["kosum"]["project_id"], rb["kosum"]["project_id"]);
    assert_eq!(ra["kosum"]["project_id"], ra2["kosum"]["project_id"]);
    assert_eq!(ra2["kosum"]["project_path"], "ekip/a-yeni-ad");

    // aynı adlı senaryonun geçmişi proje bazında ayrı
    let pid_a = ra["kosum"]["project_id"].as_i64().unwrap();
    let key = scen(&ra, "Geçerli bilgilerle giriş")["senaryo"]["scenario_key"]
        .as_str()
        .unwrap()
        .to_string();
    let hist = app
        .state
        .db
        .read(move |c| divan::history::scenario_history(c, pid_a, None, &key, 50, 0))
        .await
        .unwrap();
    assert_eq!(hist.len(), 2, "yalnızca A projesinin 2 koşumu");
    assert!(hist.iter().all(|h| h.project_id == pid_a));
}

#[tokio::test]
async fn test_sw_3_3_duplicate_result_is_counted_once() {
    let app = app();
    let (_, created) = app
        .ci_json(
            "/api/ci/runs/start",
            serde_json::json!({ "project_path": "ekip/canli", "branch": "main", "total": 7 }),
        )
        .await;
    let id = created["run_id"].as_i64().unwrap();
    let dir = sample_dir();
    let one = "d9921668-4e1d-4d2d-b703-bf04dae75cdc-result.json";
    let name = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .find(|n| n.ends_with("-result.json"))
        .unwrap_or(one.into());
    for _ in 0..3 {
        let (st, _) = app
            .ci_post(
                &format!("/api/ci/runs/{id}/results?name={name}"),
                "application/json",
                std::fs::read(dir.join(&name)).unwrap(),
            )
            .await;
        assert_eq!(st, StatusCode::OK);
    }
    assert_eq!(run_json(&app, id).await["kosum"]["total"], 1);
    // son toplu zip ile tutarlı: tekrarlar sayılmaz, toplam yine 7
    let (st, body) = app
        .ci_post(&format!("/api/ci/runs/{id}/upload"), "application/zip", zip_dir(&dir))
        .await;
    assert_eq!(st, StatusCode::OK, "{body}");
    let sum: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(sum["duplicate_results"], 1);
    let run = run_json(&app, id).await;
    assert_eq!(run["kosum"]["total"], 7);
    assert_eq!(run["kosum"]["status"], "running");
}

#[tokio::test]
async fn test_sw_4_9_rerun_last_attempt_wins_and_attempts_listed() {
    let app = app();
    let (_, sum) = upload(&app, Q, zip_dir(&sample_dir())).await;
    let run = run_json(&app, sum["run_id"].as_i64().unwrap()).await;
    let flaky = scen(&run, "Yavaş açılan karşılama mesajı");
    assert_eq!(flaky["senaryo"]["status"], "passed");
    assert_eq!(flaky["deneme_sayisi"], 2);
    let sid = flaky["senaryo"]["id"].as_i64().unwrap();
    let (_, d) = app.get(&format!("/api/scenarios/{sid}")).await;
    let d: Value = serde_json::from_str(&d).unwrap();
    let att = d["denemeler"].as_array().unwrap();
    assert_eq!(att.len(), 2);
    assert_eq!(att[0]["status"], "failed");
    assert_eq!(att[0]["attempt"], 1);
    assert_eq!(att[1]["status"], "passed");
    assert_eq!(att[1]["attempt"], 2);
}
