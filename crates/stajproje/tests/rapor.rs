//! Faz 4–6: raporlama, arama, CSV, paylaşım, canlı ekran, disk temizliği
mod util;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use util::*;

const Q: &str = "project_id=7&project_path=ekip%2Fjavafx&project_url=https%3A%2F%2Fgitlab.sirket%2Fekip%2Fjavafx&branch=main&commit=abc123def4567890&pipeline_id=9001&pipeline_url=https%3A%2F%2Fgitlab.sirket%2Fekip%2Fjavafx%2F-%2Fpipelines%2F9001";

#[tokio::test]
async fn test_sw_4_1_run_to_scenario_to_steps_and_attachments() {
    let app = app();
    let id = upload_sample(&app, Q).await;
    let (st, html) = app.get(&format!("/runs/{id}")).await;
    assert_eq!(st, StatusCode::OK);
    assert!(html.contains("Yanlış şifrede özel mesaj"));
    let run = run_json(&app, id).await;
    let sid = scenario_id(&run, "Yanlış şifrede özel mesaj");
    assert!(html.contains(&format!("href=\"/scenarios/{sid}\"")));
    let (st, page) = app.get(&format!("/scenarios/{sid}")).await;
    assert_eq!(st, StatusCode::OK);
    assert!(page.contains("O zaman &#34;Şifre hatalı&#34; mesajı görünür") || page.contains("O zaman &quot;Şifre hatalı&quot; mesajı görünür"), "adımlar görünmeli");
    assert!(page.contains("<img"), "ekran görüntüsü sayfada açılır");
    assert!(page.contains("<video"), "video sayfada açılır");
    // video range ile parça parça
    let vid = run["senaryolar"].as_array().unwrap().iter().flat_map(|s| s["ekler"].as_array().unwrap().iter()).find(|a| a["mime"] == "video/mp4").unwrap()["id"].as_i64().unwrap();
    let r = app.send(Request::get(format!("/files/{vid}")).header("range", "bytes=0-99").body(Body::empty()).unwrap()).await;
    assert_eq!(r.status(), StatusCode::PARTIAL_CONTENT);
    use http_body_util::BodyExt;
    assert_eq!(r.into_body().collect().await.unwrap().to_bytes().len(), 100);
}

#[tokio::test]
async fn test_sw_4_3_ci_info_in_report_header() {
    let app = app();
    let id = upload_sample(&app, Q).await;
    let (_, html) = app.get(&format!("/runs/{id}")).await;
    assert!(html.contains("ekip/javafx"));
    assert!(html.contains("<b>main</b>"));
    assert!(html.contains("abc123de"));
    assert!(html.contains("href=\"https://gitlab.sirket/ekip/javafx/-/pipelines/9001\""), "pipeline linki tıklanabilir");
    assert!(html.contains("href=\"https://gitlab.sirket/ekip/javafx/-/commit/abc123def4567890\""));
}

/// Senaryonun durumunu değiştirerek yeni koşum yükler.
fn mutated_zip(name: &str, status: &str) -> Vec<u8> {
    let tmp = tempfile::tempdir().unwrap();
    for e in std::fs::read_dir(sample_dir()).unwrap() {
        let p = e.unwrap().path();
        let n = p.file_name().unwrap().to_string_lossy().to_string();
        let mut data = std::fs::read(&p).unwrap();
        if n.ends_with("-result.json") {
            let mut v: serde_json::Value = serde_json::from_slice(&data).unwrap();
            if v["name"] == name {
                v["status"] = status.into();
            }
            data = v.to_string().into_bytes();
        }
        std::fs::write(tmp.path().join(&n), data).unwrap();
    }
    zip_dir(tmp.path())
}

#[tokio::test]
async fn test_sw_4_2_dashboard_history_and_flaky_detection() {
    let app = app();
    let q = "project_path=ekip%2Fflaky&branch=main";
    // 1. koşum: "Geçerli bilgilerle giriş" geçer, 2. koşum: kalır → flaky
    upload_sample(&app, q).await;
    let (st, body) = app.ci_post(&format!("/api/ci/runs?{q}"), "application/zip", mutated_zip("Geçerli bilgilerle giriş", "failed")).await;
    assert_eq!(st, StatusCode::OK, "{body}");
    let (st, dash) = app.get("/").await;
    assert_eq!(st, StatusCode::OK);
    assert!(dash.contains("Koşumlar") && dash.contains("#2"));
    assert!(dash.contains("Flaky senaryolar"));
    assert!(dash.contains("Geçerli bilgilerle giriş"));
    // rerun'da önce kalıp sonra geçen de flaky (SW-4.9 girdisi)
    assert!(dash.contains("Yavaş açılan karşılama mesajı"));
    // her zaman geçen senaryo flaky değil
    let flaky_section = dash.split("Flaky senaryolar").nth(1).unwrap();
    assert!(!flaky_section.contains("Hatalı girişlerde mesaj"));
    // geçmiş listesi
    let pid = run_json(&app, 1).await["kosum"]["project_id"].as_i64().unwrap();
    let key = run_json(&app, 1).await["senaryolar"].as_array().unwrap().iter().find(|s| s["senaryo"]["name"] == "Geçerli bilgilerle giriş").unwrap()["senaryo"]["scenario_key"].as_str().unwrap().to_string();
    let (st, hist) = app.get(&format!("/history?project={pid}&branch=main&key={key}")).await;
    assert_eq!(st, StatusCode::OK);
    assert!(hist.contains("#1") && hist.contains("#2") && hist.contains("flaky"));
}

#[tokio::test]
async fn test_sw_4_6_dashboard_filtered_by_project_and_branch() {
    let app = app();
    upload_sample(&app, "project_path=ekip%2Fa&branch=main").await;
    upload_sample(&app, "project_path=ekip%2Fb&branch=develop").await;
    let (_, all) = app.get("/").await;
    assert!(all.contains("ekip/a") && all.contains("ekip/b"));
    let (_, only_b) = app.get("/?project=2&branch=develop").await;
    let table = only_b.split("<h2>Koşumlar</h2>").nth(1).unwrap();
    assert!(table.contains("ekip/b") && !table.contains("ekip/a<"));
}

#[tokio::test]
async fn test_sw_4_7_search_by_each_field() {
    let app = app();
    let id = upload_sample(&app, Q).await;
    app.state.db.write(move |c| stajproje::triage::apply_rules(c, id, 10, 0.7).map(|_| ())).await.unwrap();
    let count = |html: &str| html.split("<h2>").nth(1).unwrap().split(' ').next().unwrap().to_string();
    for (q, n) in [
        ("tag=smoke", "1"),
        ("tag=%40giris", "7"),
        ("feature=Giri%C5%9F", "7"),
        ("doors=DOORS-1003", "1"),
        ("doors=1004", "1"),
        ("jira=STAJ-42", "1"),
        ("triage=urun", "1"),
        ("triage=test_kodu", "1"),
        ("q=sifremiUnuttum", "1"),
        ("status=kaldi", "2"),
        ("project=1&branch=main", "7"),
        ("branch=yok", "0"),
    ] {
        let (st, html) = app.get(&format!("/search?{q}")).await;
        assert_eq!(st, StatusCode::OK);
        assert_eq!(count(&html), n, "{q}");
    }
}

#[tokio::test]
async fn test_sw_4_7_search_paginated() {
    let app = app();
    for _ in 0..8 {
        upload_sample(&app, "project_path=ekip%2Fsayfa&branch=main").await;
    }
    let (_, p1) = app.get("/search?project=1").await;
    assert!(p1.contains("56 sonuç"));
    assert!(p1.contains("Sayfa 1 / 2"));
    let (_, p2) = app.get("/search?project=1&page=2").await;
    assert!(p2.contains("Sayfa 2 / 2"));
    assert_eq!(p2.matches("class=\"durum ").count(), 6);
}

#[tokio::test]
async fn test_sw_4_8_csv_selected_fields_in_order_with_bom() {
    let app = app();
    let id = upload_sample(&app, Q).await;
    let (st, csv) = app.get(&format!("/export.csv?run={id}&status=kaldi&fields=durum,senaryo,doors,jira")).await;
    assert_eq!(st, StatusCode::OK);
    assert!(csv.starts_with('\u{feff}'), "UTF-8 BOM");
    let lines: Vec<&str> = csv.trim_start_matches('\u{feff}').lines().collect();
    assert_eq!(lines[0], "Durum,Senaryo,DOORS numarası,Jira issue");
    assert_eq!(lines.len(), 3);
    assert!(lines.contains(&"failed,Yanlış şifrede özel mesaj,DOORS-1003,STAJ-42"));
    // formdan gelen alan listesi + noktalı virgül
    let (_, csv) = app.get(&format!("/export.csv?run={id}&alan=senaryo&alan=durum&sep=%3B")).await;
    assert!(csv.trim_start_matches('\u{feff}').starts_with("Senaryo;Durum\r\n"));
}

#[tokio::test]
async fn test_sw_4_8_large_csv_is_streamed() {
    let app = app();
    for _ in 0..40 {
        upload_sample(&app, "project_path=ekip%2Fbuyuk&branch=main").await;
    }
    let r = app.send(Request::get("/export.csv?project=1").body(Body::empty()).unwrap()).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert!(r.headers().get("content-length").is_none(), "akış halinde (chunked) yazılır");
    let body = body_string(r).await;
    assert_eq!(body.lines().count(), 1 + 40 * 7);
}

#[tokio::test]
async fn test_sw_6_4_doors_csv_and_dxl() {
    let app = app();
    let id = upload_sample(&app, Q).await;
    let (st, csv) = app.get(&format!("/runs/{id}/doors.csv")).await;
    assert_eq!(st, StatusCode::OK);
    assert!(csv.contains("DOORS ID,Senaryo,Feature,Sonuç,Jira,Koşum,Tarih"));
    assert!(csv.contains("DOORS-1003,Yanlış şifrede özel mesaj,Giriş ekranı,Kaldı,STAJ-42"));
    assert!(csv.contains("DOORS-1001,Geçerli bilgilerle giriş,Giriş ekranı,Geçti"));
    assert_eq!(csv.matches("DOORS-1002").count(), 2, "taslak satırları ayrı");
    let (st, dxl) = app.get(&format!("/runs/{id}/doors.dxl")).await;
    assert_eq!(st, StatusCode::OK);
    assert!(dxl.contains("Module m = current"));
    assert!(dxl.contains("\"1003\""));
    assert!(dxl.contains("\"Kaldı\""));
}

#[tokio::test]
async fn test_sw_5_1_share_link_hides_internal_info() {
    let app = app();
    let id = upload_sample(&app, Q).await;
    let cookie = app.admin_cookie();
    let r = post_form(&app, &format!("/admin/runs/{id}/share"), &cookie, "").await;
    assert_eq!(r.status(), StatusCode::SEE_OTHER);
    let loc = r.headers()["location"].to_str().unwrap().to_string();
    let token = loc.split("share:").nth(1).unwrap().to_string();
    // giriş yapmadan açılır
    let (st, html) = app.get(&format!("/s/{token}")).await;
    assert_eq!(st, StatusCode::OK);
    assert!(html.contains("Yanlış şifrede özel mesaj"));
    assert!(!html.contains("STAJ-42"), "Jira anahtarı görünmez");
    assert!(!html.contains("href=\"/files/") && !html.contains("src=\"/files/"), "ham ek linki görünmez");
    assert!(!html.contains("test-ci-token"));
    assert!(!html.contains("pipelines/9001"), "iç pipeline linki görünmez");
    assert!(html.contains(&format!("/s/{token}/files/")));
    // senaryo detayı ve ek paylaşım üzerinden
    let run = run_json(&app, id).await;
    let sid = scenario_id(&run, "Yanlış şifrede özel mesaj");
    let (st, sp) = app.get(&format!("/s/{token}/scenarios/{sid}")).await;
    assert_eq!(st, StatusCode::OK);
    assert!(!sp.contains("STAJ-42") && !sp.contains("\"/files/"));
    let att = sp.split(&format!("/s/{token}/files/")).nth(1).unwrap().split('"').next().unwrap().to_string();
    let (st, _) = app.get(&format!("/s/{token}/files/{att}")).await;
    assert_eq!(st, StatusCode::OK);
    // başka koşumun senaryosu bu tokenla açılmaz
    let id2 = upload_sample(&app, Q).await;
    let sid2 = scenario_id(&run_json(&app, id2).await, "Yanlış şifrede özel mesaj");
    let (st, _) = app.get(&format!("/s/{token}/scenarios/{sid2}")).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    assert!(!log_lines("paylaşım linki oluşturuldu").is_empty());
}

#[tokio::test]
async fn test_sw_5_2_revoked_share_link_does_not_open() {
    let app = app();
    let id = upload_sample(&app, Q).await;
    let cookie = app.admin_cookie();
    let r = post_form(&app, &format!("/admin/runs/{id}/share"), &cookie, "").await;
    let token = r.headers()["location"].to_str().unwrap().split("share:").nth(1).unwrap().to_string();
    // admin değilken iptal edilemez
    let r = post_form(&app, "/admin/shares/1/revoke", "", "").await;
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    let (_, admin) = app.get_admin("/admin").await;
    assert!(admin.contains(&token));
    let r = post_form(&app, "/admin/shares/1/revoke", &cookie, "").await;
    assert_eq!(r.status(), StatusCode::SEE_OTHER);
    let (st, _) = app.get(&format!("/s/{token}")).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    assert!(!log_lines("paylaşım linki iptal edildi").is_empty());
}

#[tokio::test]
async fn test_sw_3_4_live_snapshot_waiting_running_and_errors() {
    let app = app();
    let (_, v) = app
        .ci_json(
            "/api/ci/runs/start",
            serde_json::json!({
                "project_path": "ekip/canli", "branch": "main",
                "expected": [
                    { "name": "Yanlış şifrede özel mesaj", "feature": "Giriş ekranı" },
                    { "name": "Geçerli bilgilerle giriş", "feature": "Giriş ekranı" },
                    { "name": "Henüz koşmayan", "feature": "Giriş ekranı" },
                    { "name": "Daha sonra", "feature": "Giriş ekranı" }
                ]
            }),
        )
        .await;
    let id = v["run_id"].as_i64().unwrap();
    // kalan senaryonun sonucu + ekran görüntüsü gelir
    let dir = sample_dir();
    for n in std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().to_string()) {
        let data = std::fs::read(dir.join(&n)).unwrap();
        let s = String::from_utf8_lossy(&data);
        if n.contains("-attachment") || n.ends_with("-container.json") || s.contains("\"Yanlış şifrede özel mesaj\"") {
            app.ci_post(&format!("/api/ci/runs/{id}/results?name={n}"), "application/octet-stream", data).await;
        }
    }
    let snap = app.state.db.read(move |c| stajproje::live::snapshot(c, id)).await.unwrap().unwrap();
    assert_eq!(snap.status, "running");
    assert_eq!(snap.progress, "1/4");
    let states: Vec<&str> = snap.scenarios.iter().map(|s| s.state.as_str()).collect();
    assert_eq!(states, vec!["kaldi", "kosuyor", "bekliyor", "bekliyor"]);
    assert!(snap.scenarios[0].message.as_deref().unwrap().contains("Şifre hatalı"), "hata koşum bitmeden görünür");
    assert!(snap.scenarios[0].screenshot_id.is_some());
    let (st, html) = app.get(&format!("/runs/{id}/live")).await;
    assert_eq!(st, StatusCode::OK);
    assert!(html.contains("/ws/runs/"));
}

#[tokio::test]
async fn test_sw_3_4_websocket_pushes_updates_without_reload() {
    use futures_util::StreamExt;
    let app = app();
    let (_, v) = app.ci_json("/api/ci/runs/start", serde_json::json!({ "project_path": "ekip/ws", "branch": "main", "total": 7 })).await;
    let id = v["run_id"].as_i64().unwrap();
    let base = spawn_fake(app.router.clone()).await;
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("{}/ws/runs/{id}", base.replace("http", "ws"))).await.unwrap();
    let first: serde_json::Value = serde_json::from_str(ws.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
    assert_eq!(first["done"], 0);
    let dir = sample_dir();
    let n = "0b36ff12-0a0a-4ca4-960b-50de3d2e71b6-result.json";
    let name = if dir.join(n).exists() { n.to_string() } else { std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().to_string()).find(|x| x.ends_with("-result.json")).unwrap() };
    app.ci_post(&format!("/api/ci/runs/{id}/results?name={name}"), "application/json", std::fs::read(dir.join(&name)).unwrap()).await;
    let next = tokio::time::timeout(std::time::Duration::from_secs(3), ws.next()).await.expect("güncelleme gelmedi").unwrap().unwrap();
    let v: serde_json::Value = serde_json::from_str(next.to_text().unwrap()).unwrap();
    assert_eq!(v["done"], 1);
}

struct FakeDisk(std::sync::Mutex<Vec<f64>>);
impl stajproje::disk::DiskProbe for FakeDisk {
    fn used_pct(&self) -> anyhow::Result<f64> {
        let mut v = self.0.lock().unwrap();
        Ok(if v.len() > 1 { v.remove(0) } else { v[0] })
    }
}

#[tokio::test]
async fn test_sw_10_8_cleanup_order_videos_then_images_oldest_first() {
    let app = app();
    let id1 = upload_sample(&app, Q).await;
    let id2 = upload_sample(&app, Q).await;
    // STAJ-42 bağlantısı: ekin yerine "Silindi, bkz. STAJ-42"
    let paths = app.state.paths.clone();
    // ölçümler: başlangıç %95; koşum 1 videoları silindikten sonra hâlâ %85; koşum 2 videolarından sonra %79 → dur
    let probe = FakeDisk(std::sync::Mutex::new(vec![95.0, 95.0, 85.0, 79.0]));
    let rep = app.state.db.write(move |c| stajproje::disk::cleanup(c, &paths, &probe, 80.0, 90.0)).await.unwrap();
    assert!(rep.deleted_files >= 2);
    let (a, b) = (run_json(&app, id1).await, run_json(&app, id2).await);
    let deleted = |r: &serde_json::Value, mime: &str| {
        r["senaryolar"].as_array().unwrap().iter().flat_map(|s| s["ekler"].as_array().unwrap().iter()).filter(|x| x["mime"] == mime && !x["deleted_reason"].is_null()).count()
    };
    assert!(deleted(&a, "video/mp4") > 0, "önce en eski koşumun videoları");
    assert!(deleted(&b, "video/mp4") > 0);
    assert_eq!(deleted(&a, "image/png") + deleted(&b, "image/png"), 0, "eşik altına inince görüntülere geçmeden durur");
    // silinen ek için sayfa hata vermez, Jira numarası yazar
    let sid = scenario_id(&a, "Yanlış şifrede özel mesaj");
    let (st, page) = app.get(&format!("/scenarios/{sid}")).await;
    assert_eq!(st, StatusCode::OK);
    assert!(page.contains("Silindi, bkz. STAJ-42"));
    // kayıtlar silinmez
    assert_eq!(a["kosum"]["total"], 7);
}

#[tokio::test]
async fn test_sw_10_8_images_deleted_after_videos_when_still_full() {
    let app = app();
    upload_sample(&app, Q).await;
    let paths = app.state.paths.clone();
    let probe = FakeDisk(std::sync::Mutex::new(vec![95.0]));
    app.state.db.write(move |c| stajproje::disk::cleanup(c, &paths, &probe, 80.0, 90.0)).await.unwrap();
    let r = run_json(&app, 1).await;
    let left: usize = r["senaryolar"].as_array().unwrap().iter().flat_map(|s| s["ekler"].as_array().unwrap().iter()).filter(|x| (x["mime"] == "image/png" || x["mime"] == "video/mp4") && x["deleted_reason"].is_null()).count();
    assert_eq!(left, 0);
    // metin ekleri silinmez
    let text_left = r["senaryolar"].as_array().unwrap().iter().flat_map(|s| s["ekler"].as_array().unwrap().iter()).filter(|x| x["mime"] == "text/plain" && x["deleted_reason"].is_null()).count();
    assert!(text_left > 0);
    // %85: temizlik başlamaz
    let paths = app.state.paths.clone();
    let probe = FakeDisk(std::sync::Mutex::new(vec![85.0]));
    let rep = app.state.db.write(move |c| stajproje::disk::cleanup(c, &paths, &probe, 80.0, 90.0)).await.unwrap();
    assert_eq!(rep.deleted_files, 0);
}
