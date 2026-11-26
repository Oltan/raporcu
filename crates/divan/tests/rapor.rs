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
    assert!(
        page.contains("O zaman &#34;Şifre hatalı&#34; mesajı görünür") || page.contains("O zaman &quot;Şifre hatalı&quot; mesajı görünür"),
        "adımlar görünmeli"
    );
    assert!(page.contains("<img"), "ekran görüntüsü sayfada açılır");
    assert!(page.contains("<video"), "video sayfada açılır");
    // video range ile parça parça
    let vid = run["senaryolar"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|s| s["ekler"].as_array().unwrap().iter())
        .find(|a| a["mime"] == "video/mp4")
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    let r = app
        .send(
            Request::get(format!("/files/{vid}"))
                .header("range", "bytes=0-99")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
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
    assert!(
        html.contains("href=\"https://gitlab.sirket/ekip/javafx/-/pipelines/9001\""),
        "pipeline linki tıklanabilir"
    );
    assert!(html.contains("href=\"https://gitlab.sirket/ekip/javafx/-/commit/abc123def4567890\""));
}

#[tokio::test]
async fn test_sw_4_3_uygulama_surumu_raporda_aramada_csvde() {
    let app = app();
    let id = upload_sample(&app, &format!("{Q}&app_version=1.5.0")).await;
    let eski = upload_sample(&app, Q).await;
    let (_, html) = app.get(&format!("/runs/{id}")).await;
    assert!(html.contains("Uygulama <b>1.5.0</b>"), "koşum başlığında");
    let (_, eski_html) = app.get(&format!("/runs/{eski}")).await;
    assert!(
        eski_html.contains("UYGULAMA_SURUMU verilmedi") && !eski_html.contains("<b>1.5.0</b>"),
        "sürümsüz koşumda yer görünür, değer —"
    );
    let (_, pano) = app.get("/panel").await;
    assert!(pano.contains("<th>Uygulama</th>"), "dashboard'da ayrı sütun");
    assert!(pano.contains("<b>1.5.0</b>"), "dashboard koşum listesinde");
    assert!(pano.contains("UYGULAMA_SURUMU verilmedi"), "sürümsüz koşum satırında —");
    // arama: yalnızca o sürümün koşumu
    let (_, ara) = app.get("/search?uygulama=1.5.0&status=kaldi").await;
    assert!(
        ara.contains(&format!("/runs/{id}")) && !ara.contains(&format!("/runs/{eski}\"")),
        "{ara}"
    );
    let (_, csv) = app.get("/export.csv?uygulama=1.5.0&fields=kosum,senaryo,uygulama").await;
    let satirlar: Vec<&str> = csv.trim_start_matches('\u{feff}').lines().collect();
    assert_eq!(satirlar[0], "Koşum,Senaryo,Uygulama sürümü");
    assert!(
        satirlar.len() > 1
            && satirlar[1..]
                .iter()
                .all(|l| l.starts_with(&format!("{id},")) && l.ends_with(",1.5.0")),
        "{csv}"
    );
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
    let (st, body) = app
        .ci_post(
            &format!("/api/ci/runs?{q}"),
            "application/zip",
            mutated_zip("Geçerli bilgilerle giriş", "failed"),
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{body}");
    // geçerken kalmaya başlayan senaryo bozulmadır, flaky değil
    let (_, dash) = app.get("/panel").await;
    let flaky_section = dash.split("Flaky senaryolar").nth(1).unwrap();
    assert!(!flaky_section.contains("Geçerli bilgilerle giriş"), "bozulma flaky sayılmaz");
    // 3. koşum: aynı sürümde yeniden geçer → kaldıktan sonra geçti, flaky
    upload_sample(&app, q).await;
    let (st, dash) = app.get("/panel").await;
    assert_eq!(st, StatusCode::OK);
    assert!(dash.contains("Koşumlar") && dash.contains("#2"));
    assert!(dash.contains("Flaky senaryolar"));
    let flaky_section = dash.split("Flaky senaryolar").nth(1).unwrap();
    assert!(flaky_section.contains("Geçerli bilgilerle giriş"));
    // rerun'da önce kalıp sonra geçen de flaky (SW-4.9 girdisi)
    assert!(dash.contains("Yavaş açılan karşılama mesajı"));
    // her zaman geçen senaryo flaky değil
    let flaky_section = dash.split("Flaky senaryolar").nth(1).unwrap();
    assert!(!flaky_section.contains("Hatalı girişlerde mesaj"));
    // geçmiş listesi
    let pid = run_json(&app, 1).await["kosum"]["project_id"].as_i64().unwrap();
    let key = run_json(&app, 1).await["senaryolar"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["senaryo"]["name"] == "Geçerli bilgilerle giriş")
        .unwrap()["senaryo"]["scenario_key"]
        .as_str()
        .unwrap()
        .to_string();
    let (st, hist) = app.get(&format!("/history?project={pid}&branch=main&key={key}")).await;
    assert_eq!(st, StatusCode::OK);
    assert!(hist.contains("#1") && hist.contains("#2") && hist.contains("flaky"));
}

/// Flaky: kod değişmeden (aynı commit) kaldıktan sonra geçmek. Commit değişince geçen senaryo düzeltilmiştir;
/// geçerken kalmaya başlayan senaryo bozulmadır.
#[tokio::test]
async fn test_sw_4_2_flaky_needs_pass_after_fail_on_same_commit() {
    let app = app();
    let ad = "Geçerli bilgilerle giriş";
    let kaldi = |q: String| {
        let app = &app;
        async move {
            let (st, body) = app
                .ci_post(&format!("/api/ci/runs?{q}"), "application/zip", mutated_zip(ad, "failed"))
                .await;
            assert_eq!(st, StatusCode::OK, "{body}");
        }
    };
    let flaky = |proje: &'static str| {
        let app = &app;
        async move {
            app.state
                .db
                .read(move |c| {
                    let (pid, key): (i64, String) = c.query_row(
                        "SELECT s.project_id, s.scenario_key FROM scenarios s JOIN projects p ON p.id = s.project_id
                          WHERE p.path = ?1 AND s.name = ?2 LIMIT 1",
                        rusqlite::params![proje, ad],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )?;
                    divan::history::is_flaky(c, pid, "main", &key, 10)
                })
                .await
                .unwrap()
        }
    };
    // commit aaa'da kaldı, commit bbb'de geçti: düzeltildi
    kaldi("project_path=ekip%2Fduzeltildi&branch=main&commit=aaa".into()).await;
    upload_sample(&app, "project_path=ekip%2Fduzeltildi&branch=main&commit=bbb").await;
    assert!(!flaky("ekip/duzeltildi").await, "commit değişince geçen senaryo flaky değil");
    // aynı commit'te geçti, sonra iki kez kaldı: bozulma
    upload_sample(&app, "project_path=ekip%2Fbozuldu&branch=main&commit=aaa").await;
    kaldi("project_path=ekip%2Fbozuldu&branch=main&commit=aaa".into()).await;
    kaldi("project_path=ekip%2Fbozuldu&branch=main&commit=aaa".into()).await;
    assert!(!flaky("ekip/bozuldu").await, "kaldıktan sonra hiç geçmeyen senaryo flaky değil");
    // aynı commit'te kaldı, sonra geçti: flaky; yeniden kalınca da flaky kalır
    kaldi("project_path=ekip%2Fdegisken&branch=main&commit=aaa".into()).await;
    upload_sample(&app, "project_path=ekip%2Fdegisken&branch=main&commit=aaa").await;
    assert!(flaky("ekip/degisken").await);
    kaldi("project_path=ekip%2Fdegisken&branch=main&commit=aaa".into()).await;
    assert!(flaky("ekip/degisken").await);
}

#[tokio::test]
async fn test_sw_4_6_dashboard_filtered_by_project_and_branch() {
    let app = app();
    upload_sample(&app, "project_path=ekip%2Fa&branch=main").await;
    upload_sample(&app, "project_path=ekip%2Fb&branch=develop").await;
    let (_, all) = app.get("/panel").await;
    assert!(all.contains("ekip/a") && all.contains("ekip/b"));
    let (_, only_b) = app.get("/panel?project=2&branch=develop").await;
    let table = only_b.split("<h2>Koşumlar</h2>").nth(1).unwrap();
    assert!(table.contains("ekip/b") && !table.contains("ekip/a<"));
}

#[tokio::test]
async fn test_sw_4_7_search_by_each_field() {
    let app = app();
    let id = upload_sample(&app, Q).await;
    app.state
        .db
        .write(move |c| divan::triage::apply_rules(c, id, 10, 0.7).map(|_| ()))
        .await
        .unwrap();
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
    let (st, csv) = app
        .get(&format!("/export.csv?run={id}&status=kaldi&fields=durum,senaryo,doors,jira"))
        .await;
    assert_eq!(st, StatusCode::OK);
    assert!(csv.starts_with('\u{feff}'), "UTF-8 BOM");
    let lines: Vec<&str> = csv.trim_start_matches('\u{feff}').lines().collect();
    assert_eq!(lines[0], "Durum,Senaryo,DOORS,Jira issue");
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
    // DXL varsayılan kapalı: düğme yok, uç 404
    let off = app();
    let id0 = upload_sample(&off, Q).await;
    let (_, page) = off.get(&format!("/runs/{id0}")).await;
    assert!(page.contains("DOORS CSV") && !page.contains("doors.dxl"));
    assert_eq!(off.get(&format!("/runs/{id0}/doors.dxl")).await.0, StatusCode::NOT_FOUND);
    let app = app_with(|c| c.doors_dxl = true);
    let id = upload_sample(&app, Q).await;
    let (st, csv) = app.get(&format!("/runs/{id}/doors.csv")).await;
    assert_eq!(st, StatusCode::OK);
    assert!(csv.contains("DOORS,Senaryo,Feature,Sonuç,Jira,Koşum,Tarih"));
    assert!(csv.contains("DOORS-1003,Yanlış şifrede özel mesaj,Giriş ekranı,Kaldı,STAJ-42"));
    assert!(csv.contains("DOORS-1001,Geçerli bilgilerle giriş,Giriş ekranı,Geçti"));
    assert_eq!(csv.matches("DOORS-1002").count(), 2, "taslak satırları ayrı");
    let (st, dxl) = app.get(&format!("/runs/{id}/doors.dxl")).await;
    assert_eq!(st, StatusCode::OK);
    assert!(dxl.contains("m = current"), "modül yolu yoksa açık modül");
    assert!(dxl.contains("string idAttr = \"Absolute Number\""));
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
    // SW-6.2: bağlı issue'lar ekip linkinde de görünür, ama Jira'ya link yok
    assert!(html.contains("Jira issue'ları") && html.contains("STAJ-42"), "Jira özeti görünür");
    assert!(!html.contains("/browse/"), "Jira'ya link yok");
    assert!(
        !html.contains("href=\"/files/") && !html.contains("src=\"/files/"),
        "ham ek linki görünmez"
    );
    assert!(!html.contains("test-ci-token"));
    assert!(!html.contains("pipelines/9001"), "iç pipeline linki görünmez");
    assert!(html.contains(&format!("/s/{token}/files/")));
    // senaryo detayı ve ek paylaşım üzerinden
    let run = run_json(&app, id).await;
    let sid = scenario_id(&run, "Yanlış şifrede özel mesaj");
    let (st, sp) = app.get(&format!("/s/{token}/scenarios/{sid}")).await;
    assert_eq!(st, StatusCode::OK);
    assert!(!sp.contains("STAJ-42") && !sp.contains("\"/files/"));
    let att = sp
        .split(&format!("/s/{token}/files/"))
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_string();
    let (st, _) = app.get(&format!("/s/{token}/files/{att}")).await;
    assert_eq!(st, StatusCode::OK);
    // başka koşumun senaryosu bu tokenla açılmaz
    let id2 = upload_sample(&app, Q).await;
    let sid2 = scenario_id(&run_json(&app, id2).await, "Yanlış şifrede özel mesaj");
    let (st, _) = app.get(&format!("/s/{token}/scenarios/{sid2}")).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    assert!(!log_lines("paylaşım linki oluşturuldu").is_empty());
}

/// Örnek sonuçların ikinci bir feature olarak koşulmuş kopyası (`Ayarlar`, farklı anahtarlar), ve
/// istenirse feature dosyası olmayan bir JUnit/ortam yoklama sonucu.
fn feature_kopyasi(feature: Option<&str>, probe: bool) -> Vec<u8> {
    let mut girdiler: Vec<(String, Vec<u8>)> = Vec::new();
    for e in std::fs::read_dir(sample_dir()).unwrap() {
        let p = e.unwrap().path();
        let n = p.file_name().unwrap().to_string_lossy().to_string();
        let mut data = std::fs::read(&p).unwrap();
        if let Some(f) = feature {
            if n.ends_with(".json") {
                let s = String::from_utf8(data)
                    .unwrap()
                    .replace("\"Giriş ekranı\"", &format!("\"{f}\""))
                    .replace("features/giris.feature", "features/ayarlar.feature")
                    .replace("\"historyId\":\"", "\"historyId\":\"ay-")
                    .replace("\"testCaseId\":\"", "\"testCaseId\":\"ay-");
                data = s.into_bytes();
            }
            // aynı result uuid'leri ikinci kez sayılmasın diye dosya adı ve uuid değişir
            let n2 = format!("ay-{n}");
            let s = String::from_utf8_lossy(&data).replace("\"uuid\":\"", "\"uuid\":\"ay-").into_bytes();
            girdiler.push((n2, s));
        } else {
            girdiler.push((n, data));
        }
    }
    if probe {
        let v = serde_json::json!({
            "uuid": "probe-1", "historyId": "probe-hist", "name": "AwtDisplayProbeTest",
            "fullName": "core.HeadlessEnvironmentProbe.AwtDisplayProbeTest", "status": "failed",
            "statusDetails": { "message": "Headless ortam" }, "stage": "finished",
            "start": 1, "stop": 2, "labels": [{ "name": "framework", "value": "junit-platform" }]
        });
        girdiler.push(("probe-1-result.json".into(), v.to_string().into_bytes()));
    }
    let refs: Vec<(&str, &[u8])> = girdiler.iter().map(|(n, d)| (n.as_str(), d.as_slice())).collect();
    zip_entries(&refs)
}

async fn yukle_zip(app: &TestApp, q: &str, zip: Vec<u8>) -> i64 {
    let (st, b) = app.ci_post(&format!("/api/ci/runs?{q}"), "application/zip", zip).await;
    assert_eq!(st, StatusCode::OK, "{b}");
    serde_json::from_str::<serde_json::Value>(&b).unwrap()["run_id"].as_i64().unwrap()
}

#[tokio::test]
async fn test_sw_5_1_yayin_iki_kosum_tek_raporda_ve_ana_sayfa() {
    let app = app();
    let k = format!("{Q}&app_name=Kontrol%20Yaz%C4%B1l%C4%B1m%C4%B1&app_version=2.1.0");
    let r1 = yukle_zip(&app, &k, feature_kopyasi(None, true)).await;
    let r2 = yukle_zip(&app, &k, feature_kopyasi(Some("Ayarlar"), false)).await;
    let sim = yukle_zip(
        &app,
        &format!("{Q}&app_name=Sim%C3%BClat%C3%B6r&app_version=1.0"),
        feature_kopyasi(None, false),
    )
    .await;
    let cookie = app.admin_cookie();

    // yayınlanmadan ana sayfa boş; panel girişsiz (EKIP_SIFRESI yok) açık
    let (st, ana) = app.get("/").await;
    assert_eq!(st, StatusCode::OK);
    assert!(ana.contains("Henüz yayınlanmış rapor yok"));

    // admin: aday listesi ve form (koşumun üçlüsüyle)
    let (_, liste) = app.get_admin("/admin/yayinlar").await;
    assert!(liste.contains("Kontrol Yazılımı") && liste.contains("Simülatör") && liste.contains("2.1.0"));
    let (st, form) = app.get_admin(&format!("/admin/yayinlar/yeni?run={r1}")).await;
    assert_eq!(st, StatusCode::OK);
    assert!(form.contains(&format!("value=\"{r1}\" checked")) && form.contains(&format!("value=\"{r2}\" checked")));
    assert!(!form.contains(&format!("value=\"{sim}\"")), "başka yazılımın koşumu listede yok");
    assert!(form.contains("1 sonuç (JUnit"), "yoklama testi kendiliğinden dışarıda");
    let (st, _) = app.get("/admin/yayinlar/yeni?run=1").await;
    assert_ne!(st, StatusCode::OK, "admin girişi gerekir");

    // bir testi rapordan çıkararak yayınla
    let haric: String = app
        .state
        .db
        .read(move |c| {
            Ok(c.query_row(
                "SELECT scenario_key FROM scenarios WHERE run_id = ?1 AND name = 'Geçerli bilgilerle giriş'",
                [r1],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap();
    let govde = format!(
        "proje=1&yazilim=Kontrol%20Yaz%C4%B1l%C4%B1m%C4%B1&surum=2.1.0&kosum={r1}&kosum={r2}&haric={}",
        urlencoding::encode(&haric)
    );
    let r = post_form(&app, "/admin/yayinlar", &cookie, &govde).await;
    assert_eq!(r.status(), StatusCode::SEE_OTHER);
    let yer = r.headers()["location"].to_str().unwrap().to_string();
    assert!(yer.starts_with("/rapor/"));
    let r = post_form(
        &app,
        "/admin/yayinlar",
        &cookie,
        &format!("proje=1&yazilim=Sim%C3%BClat%C3%B6r&surum=1.0&kosum={sim}"),
    )
    .await;
    assert_eq!(r.status(), StatusCode::SEE_OTHER);
    assert!(!log_lines("rapor yayınlandı").is_empty());

    // test raporu: girişsiz, iki feature tek raporda
    let (st, rapor) = app.get(&yer).await;
    assert_eq!(st, StatusCode::OK);
    assert!(rapor.contains("Kontrol Yazılımı · sürüm 2.1.0"));
    assert!(
        rapor.contains("Giriş ekranı") && rapor.contains("Ayarlar"),
        "iki koşumun feature'ları birleşti"
    );
    // ikinci denemede geçen test görünür ve "Geçti" (eski hata: hiç görünmüyor, inceleniyor sayılıyordu)
    let i = rapor.find("Yavaş açılan karşılama mesajı").expect("tekrarda geçen test raporda");
    assert!(rapor[i.saturating_sub(200)..i].contains("Geçti"), "tekrarda geçen = Geçti");
    // giriş: 2 hatalı giriş + yavaş = 3 geçti (geçerli giriş çıkarıldı), 2 kaldı; ayarlar: 4 geçti, 2 kaldı
    assert!(rapor.contains("<b>11</b><span>Toplam test</span>"), "toplam");
    assert!(rapor.contains("<b>7</b><span>Geçen test</span>") && rapor.contains("<b>4</b><span>Kalan test</span>"));
    assert!(rapor.contains("Başarı oranı") && rapor.contains("<details class=\"test-case"));
    for yok in [
        "nceleniyor",
        "AwtDisplayProbeTest",
        "abc123de",
        "Commit",
        "Branch",
        "Koşum #",
        "Ürün hataları",
        "?bolum=",
        "/browse/",
        "Doğrulan",
    ] {
        assert!(!rapor.contains(yok), "raporda olmamalı: {yok}");
    }
    assert!(rapor.contains("STAJ-42"), "Jira issue'su test case'in içinde");
    assert_eq!(
        rapor.matches("Geçerli bilgilerle giriş").count(),
        1,
        "çıkarılan test yalnızca diğer feature'da"
    );

    // test ayrıntısı ve ekran görüntüsü yalnızca rapordaki testler için
    let id: i64 = yer.trim_start_matches("/rapor/").parse().unwrap();
    let (kalan, probe, disari): (i64, i64, i64) = app
        .state
        .db
        .read(move |c| {
            let q = |n: &str, r: i64| {
                c.query_row(
                    "SELECT id FROM scenarios WHERE run_id = ?1 AND name = ?2 AND is_latest = 1",
                    rusqlite::params![r, n],
                    |x| x.get::<_, i64>(0),
                )
            };
            Ok((
                q("Yanlış şifrede özel mesaj", r1)?,
                q("AwtDisplayProbeTest", r1)?,
                q("Yanlış şifrede özel mesaj", sim)?,
            ))
        })
        .await
        .unwrap();
    let (st, ayrinti) = app.get(&format!("/rapor/{id}/test/{kalan}")).await;
    assert_eq!(st, StatusCode::OK);
    assert!(ayrinti.contains("Adımlar") && ayrinti.contains("Beklenen ve görülen") && !ayrinti.contains("abc123de"));
    assert_eq!(app.get(&format!("/rapor/{id}/test/{probe}")).await.0, StatusCode::NOT_FOUND);
    assert_eq!(app.get(&format!("/rapor/{id}/test/{disari}")).await.0, StatusCode::NOT_FOUND);
    let aid: i64 = app
        .state
        .db
        .read(move |c| {
            Ok(c.query_row(
                "SELECT id FROM attachments WHERE scenario_id = ?1 AND mime LIKE 'image/%' LIMIT 1",
                [kalan],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(app.get(&format!("/rapor/{id}/dosya/{aid}")).await.0, StatusCode::OK);

    // ana sayfa: yazılım sekmeleri, sürüm başına grafik, raporlar; commit ve inceleniyor yok
    let (_, ana) = app.get("/").await;
    assert!(ana.contains("Kontrol Yazılımı") && ana.contains("Simülatör"), "sekmeler");
    assert!(ana.contains("Sürümlere göre başarı") && ana.contains("Sürüm başına test sonuçları"));
    assert!(ana.contains("Toplam test") && ana.contains("Başarı oranı"));
    for yok in ["nceleniyor", "Bölümlere göre", "Commit", "ürün hatası bulundu", "Doğrulan"] {
        assert!(!ana.contains(yok), "ana sayfada olmamalı: {yok}");
    }
    let (_, sim_ana) = app.get("/?yazilim=Sim%C3%BClat%C3%B6r").await;
    assert!(sim_ana.contains("sürüm <b>1.0</b>") && !sim_ana.contains("2.1.0"));

    // eski genel linkler ana sayfaya
    let r = app
        .send(
            axum::http::Request::get("/g/eski-token/runs/1")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(r.headers()["location"], "/");

    // kaldırınca ana sayfadan çıkar
    let r = post_form(&app, &format!("/admin/yayinlar/{id}/sil"), &cookie, "").await;
    assert_eq!(r.status(), StatusCode::SEE_OTHER);
    assert_eq!(app.get(&yer).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_sw_9_5_ekip_paneli_sifreyle() {
    let app = app_with(|c| c.ekip_sifresi = Some(divan::config::Secret::new("ekip-gizli")));
    let id = upload_sample(&app, Q).await;
    let html = |uri: &str, cookie: &str| {
        axum::http::Request::get(uri)
            .header("accept", "text/html")
            .header("cookie", cookie)
            .body(axum::body::Body::empty())
            .unwrap()
    };
    // girişsiz: panel ve iç sayfalar giriş sayfasına, API 401; dışa açık sayfa açık
    for yol in ["/panel".to_string(), format!("/runs/{id}"), "/search".to_string()] {
        let r = app.send(html(&yol, "")).await;
        assert!(r.status().is_redirection(), "{yol}");
        assert!(r.headers()["location"].to_str().unwrap().starts_with("/login?next="));
    }
    assert_eq!(app.get(&format!("/api/runs/{id}")).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(app.get("/").await.0, StatusCode::OK);
    // yanlış şifre
    let r = post_form(&app, "/login", "", "password=yanlis&next=%2Fpanel").await;
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    // ekip şifresi: ekip oturumu; admin işleri yine kapalı (hatalı denemeden sonraki bekleme geçsin)
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let r = post_form(&app, "/login", "", "password=ekip-gizli&next=%2Fpanel").await;
    assert!(r.status().is_redirection());
    let cerez = r.headers()["set-cookie"].to_str().unwrap().split(';').next().unwrap().to_string();
    assert!(cerez.starts_with("divan_ekip="));
    let r = app.send(html("/panel", &cerez)).await;
    assert_eq!(r.status(), StatusCode::OK);
    let r = app.send(html(&format!("/runs/{id}"), &cerez)).await;
    assert_eq!(r.status(), StatusCode::OK);
    let r = app.send(html("/admin", &cerez)).await;
    assert!(r.status().is_redirection(), "ekip oturumu admin değildir");
    // admin oturumu ekip sayılır
    let r = app.send(html("/panel", &app.admin_cookie())).await;
    assert_eq!(r.status(), StatusCode::OK);
    // şifre loglara yazılmaz
    assert!(!divan::logging::test_capture()
        .lock()
        .unwrap()
        .iter()
        .any(|l| l.contains("ekip-gizli")));
}

#[tokio::test]
async fn test_sw_5_2_revoked_share_link_does_not_open() {
    let app = app();
    let id = upload_sample(&app, Q).await;
    let cookie = app.admin_cookie();
    let r = post_form(&app, &format!("/admin/runs/{id}/share"), &cookie, "").await;
    let token = r.headers()["location"]
        .to_str()
        .unwrap()
        .split("share:")
        .nth(1)
        .unwrap()
        .to_string();
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
    for n in std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
    {
        let data = std::fs::read(dir.join(&n)).unwrap();
        let s = String::from_utf8_lossy(&data);
        if n.contains("-attachment") || n.ends_with("-container.json") || s.contains("\"Yanlış şifrede özel mesaj\"") {
            app.ci_post(&format!("/api/ci/runs/{id}/results?name={n}"), "application/octet-stream", data)
                .await;
        }
    }
    let snap = app
        .state
        .db
        .read(move |c| divan::live::snapshot(c, id, None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(snap.status, "running");
    assert_eq!(snap.progress, "1/4");
    let states: Vec<&str> = snap.scenarios.iter().map(|s| s.state.as_str()).collect();
    assert_eq!(states, vec!["kaldi", "kosuyor", "bekliyor", "bekliyor"]);
    assert!(
        snap.scenarios[0].message.as_deref().unwrap().contains("Şifre hatalı"),
        "hata koşum bitmeden görünür"
    );
    assert!(snap.scenarios[0].screenshot_id.is_some());
    let (st, html) = app.get(&format!("/runs/{id}/live")).await;
    assert_eq!(st, StatusCode::OK);
    assert!(html.contains("/ws/runs/"));
}

#[tokio::test]
async fn test_sw_3_4_canli_entry_finds_active_run_and_ignores_stale() {
    let app = app();
    let (st, html) = app.get("/canli").await;
    assert_eq!(st, StatusCode::OK);
    assert!(html.contains("/api/canli"));
    let (_, v) = app.get("/api/canli").await;
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&v).unwrap()["run_id"],
        serde_json::Value::Null
    );

    let (_, v) = app
        .ci_json(
            "/api/ci/runs/start",
            serde_json::json!({ "project_path": "ekip/canli", "branch": "main", "total": 7 }),
        )
        .await;
    let id = v["run_id"].as_i64().unwrap();
    let (_, v) = app.get("/api/canli").await;
    assert_eq!(serde_json::from_str::<serde_json::Value>(&v).unwrap()["run_id"], id);

    // yarım kalmış, zaman aşımını bekleyen koşum bekleme sayfasını yanlış yere götürmez
    app.state
        .db
        .write(move |c| {
            c.execute("UPDATE runs SET last_activity_at = '2020-01-01T00:00:00.000Z' WHERE id = ?1", [id])?;
            Ok(())
        })
        .await
        .unwrap();
    let (_, v) = app.get("/api/canli").await;
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&v).unwrap()["run_id"],
        serde_json::Value::Null
    );
}

#[tokio::test]
async fn test_sw_3_4_websocket_pushes_updates_without_reload() {
    use futures_util::StreamExt;
    let app = app();
    let (_, v) = app
        .ci_json(
            "/api/ci/runs/start",
            serde_json::json!({ "project_path": "ekip/ws", "branch": "main", "total": 7 }),
        )
        .await;
    let id = v["run_id"].as_i64().unwrap();
    let base = spawn_fake(app.router.clone()).await;
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("{}/ws/runs/{id}", base.replace("http", "ws")))
        .await
        .unwrap();
    let first: serde_json::Value = serde_json::from_str(ws.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
    assert_eq!(first["done"], 0);
    let dir = sample_dir();
    let n = "0b36ff12-0a0a-4ca4-960b-50de3d2e71b6-result.json";
    let name = if dir.join(n).exists() {
        n.to_string()
    } else {
        std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .find(|x| x.ends_with("-result.json"))
            .unwrap()
    };
    app.ci_post(
        &format!("/api/ci/runs/{id}/results?name={name}"),
        "application/json",
        std::fs::read(dir.join(&name)).unwrap(),
    )
    .await;
    let next = tokio::time::timeout(std::time::Duration::from_secs(3), ws.next())
        .await
        .expect("güncelleme gelmedi")
        .unwrap()
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(next.to_text().unwrap()).unwrap();
    assert_eq!(v["done"], 1);
}

struct FakeDisk(std::sync::Mutex<Vec<f64>>);
impl divan::disk::DiskProbe for FakeDisk {
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
    let rep = app
        .state
        .db
        .write(move |c| divan::disk::cleanup(c, &paths, &probe, 80.0, 90.0))
        .await
        .unwrap();
    assert!(rep.deleted_files >= 2);
    let (a, b) = (run_json(&app, id1).await, run_json(&app, id2).await);
    let deleted = |r: &serde_json::Value, mime: &str| {
        r["senaryolar"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|s| s["ekler"].as_array().unwrap().iter())
            .filter(|x| x["mime"] == mime && !x["deleted_reason"].is_null())
            .count()
    };
    assert!(deleted(&a, "video/mp4") > 0, "önce en eski koşumun videoları");
    assert!(deleted(&b, "video/mp4") > 0);
    assert_eq!(
        deleted(&a, "image/png") + deleted(&b, "image/png"),
        0,
        "eşik altına inince görüntülere geçmeden durur"
    );
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
    app.state
        .db
        .write(move |c| divan::disk::cleanup(c, &paths, &probe, 80.0, 90.0))
        .await
        .unwrap();
    let r = run_json(&app, 1).await;
    let left: usize = r["senaryolar"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|s| s["ekler"].as_array().unwrap().iter())
        .filter(|x| (x["mime"] == "image/png" || x["mime"] == "video/mp4") && x["deleted_reason"].is_null())
        .count();
    assert_eq!(left, 0);
    // metin ekleri silinmez
    let text_left = r["senaryolar"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|s| s["ekler"].as_array().unwrap().iter())
        .filter(|x| x["mime"] == "text/plain" && x["deleted_reason"].is_null())
        .count();
    assert!(text_left > 0);
    // %85: temizlik başlamaz
    let paths = app.state.paths.clone();
    let probe = FakeDisk(std::sync::Mutex::new(vec![85.0]));
    let rep = app
        .state
        .db
        .write(move |c| divan::disk::cleanup(c, &paths, &probe, 80.0, 90.0))
        .await
        .unwrap();
    assert_eq!(rep.deleted_files, 0);
}

#[tokio::test]
async fn test_sw_4_1_run_grouped_by_feature_with_charts() {
    let app = app();
    let id = upload_sample(&app, Q).await;
    let (_, html) = app.get(&format!("/runs/{id}")).await;
    // koşum → feature (durum çubuğu, sayılar) → senaryolar
    assert!(html.contains("class=\"donut\""), "durum dağılımı halka grafiği");
    assert!(html.contains("Feature'lar"));
    assert!(
        html.contains("<details class=\"feature-grup\" id=\"f-giris-ekrani\" open>"),
        "kalanı olan feature açık gelir"
    );
    assert!(html.contains("4/7 geçti"));
    assert!(html.contains("href=\"#f-giris-ekrani\""));
    // dashboard: halka, gidişat çizgisi, sütunlar, feature tablosu
    upload_sample(&app, Q).await;
    let (_, d) = app.get("/panel?project=1&branch=main").await;
    for needle in [
        "class=\"donut\"",
        "class=\"cizgi-grafik\"",
        "class=\"sutun-grafik\"",
        "Feature bazında",
        "class=\"mini-cizgi\"",
        "/static/charts.js",
    ] {
        assert!(d.contains(needle), "{needle}");
    }
    let (st, js) = app.get("/static/charts.js").await;
    assert_eq!(st, StatusCode::OK);
    assert!(js.contains("textContent"));
}

#[tokio::test]
async fn test_sw_4_2_feature_history_matrix() {
    let app = app();
    let q = "project_path=ekip%2Fmatris&branch=main";
    upload_sample(&app, q).await;
    let (st, body) = app
        .ci_post(
            &format!("/api/ci/runs?{q}"),
            "application/zip",
            mutated_zip("Geçerli bilgilerle giriş", "failed"),
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{body}");
    upload_sample(&app, q).await; // kaldıktan sonra yeniden geçti → flaky
    let (st, html) = app.get("/feature?project=1&branch=main&name=Giri%C5%9F%20ekran%C4%B1").await;
    assert_eq!(st, StatusCode::OK);
    assert!(html.contains("Senaryo × koşum"));
    assert!(html.contains("class=\"cizgi-grafik\"") && html.contains("class=\"sutun-grafik\""));
    // her senaryo için iki koşum hücresi; değişken senaryo flaky
    assert!(html.contains("data-tip=\"Koşum #2: kaldı\""));
    assert!(html.contains("data-tip=\"Koşum #1: geçti\""));
    let row = html.split("Geçerli bilgilerle giriş").nth(1).unwrap();
    assert!(row[..400].contains("flaky"));
    // rapordan ve dashboard'dan bağlantı
    let (_, run) = app.get("/runs/2").await;
    assert!(
        run.contains("/feature?project=1&amp;branch=main&amp;name=Giri%C5%9F%20ekran%C4%B1")
            || run.contains("/feature?project=1&branch=main&name=Giri%C5%9F%20ekran%C4%B1")
    );
}

fn real_events() -> Vec<serde_json::Value> {
    let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/canli/ornek-giris-olaylar.jsonl");
    std::fs::read_to_string(p)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[tokio::test]
async fn test_sw_3_4_live_current_scenario_and_step_from_cucumber_events() {
    let app = app();
    let (_, v) = app
        .ci_json(
            "/api/ci/runs/start",
            serde_json::json!({ "project_path": "ekip/canli-adim", "branch": "main", "expected": [
                { "name": "Geçerli bilgilerle giriş", "feature": "Giriş ekranı" },
                { "name": "Hatalı girişlerde mesaj", "feature": "Giriş ekranı" },
                { "name": "Hatalı girişlerde mesaj", "feature": "Giriş ekranı" }
            ]}),
        )
        .await;
    let id = v["run_id"].as_i64().unwrap();
    let ev = real_events();
    // ilk senaryo bitti, ikinci senaryonun 2. adımı koşuyor
    let second_start = ev
        .iter()
        .enumerate()
        .filter(|(_, e)| e["tur"] == "senaryo_basladi")
        .nth(1)
        .unwrap()
        .0;
    let upto = second_start + 3; // basladi, adim1 basladi, adim1 bitti, adim2 basladi
    let (st, _) = app
        .ci_json(&format!("/api/ci/runs/{id}/events"), serde_json::Value::Array(ev[..=upto].to_vec()))
        .await;
    assert_eq!(st, StatusCode::NO_CONTENT);
    let (_, live) = app.get(&format!("/api/runs/{id}/live")).await;
    let live: serde_json::Value = serde_json::from_str(&live).unwrap();
    let cur = &live["current"];
    assert_eq!(cur["active"], true);
    assert_eq!(cur["name"], "Hatalı girişlerde mesaj");
    assert_eq!(cur["feature"], "Giriş ekranı");
    assert_eq!(cur["step_no"], 2);
    assert!(cur["step"].as_str().unwrap().starts_with("Eğer ki kullanıcı adı"), "{cur}");
    assert_eq!(cur["steps"][0]["status"], "PASSED");
    // kesin bilgi: ikinci beklenen senaryo "koşuyor" (tahmin değil)
    let states: Vec<&str> = live["scenarios"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["state"].as_str().unwrap())
        .collect();
    assert_eq!(
        states,
        vec!["gecti", "kosuyor", "bekliyor"],
        "ilk senaryo olaya göre bitti (dosyası gelmeden), ikinci koşuyor"
    );
    // koşum bitince durum silinir
    app.ci_json(&format!("/api/ci/runs/{id}/finish"), serde_json::json!({})).await;
    let (_, live) = app.get(&format!("/api/runs/{id}/live")).await;
    assert!(live.contains("\"current\":null"));
    // bitmiş koşuma olay: yok sayılır
    let (st, _) = app
        .ci_json(&format!("/api/ci/runs/{id}/events"), serde_json::Value::Array(ev[..2].to_vec()))
        .await;
    assert_eq!(st, StatusCode::ACCEPTED);
}
