//! Gereksinim öneki (`GEREKSINIM_ONEKI=YTET_`), ek kimlikler (`EK_KIMLIK_ONEKLERI=YGÖ-`) ve
//! Jira tag projeleri (`JIRA_TAG_PROJELERI=STAJ`: `@STAJ-42` Jira issue'su).
//!
//! Önek süreç genelinde ayarlandığı için bu dosya ayrı bir test ikilisidir; buradaki tüm
//! uygulamalar aynı önekle kurulur. Veri: gerçek örnek koşum (`testdata/allure/ornek-giris`),
//! `@DOORS-…` tag'leri `@YTET_…` yapılarak ve bir senaryoya `@YGÖ-12` eklenerek.

mod util;

use axum::http::StatusCode;
use util::*;

const Q: &str = "project_id=7&project_path=ekip%2Fjavafx&branch=main&commit=abc123def4567890&pipeline_id=9001";
const KALAN: &str = "Yanlış şifrede özel mesaj";

fn uygulama() -> TestApp {
    app_with(|c| c.onekler = common::kimlik::Onekler::parse(Some("YTET_"), Some("YGÖ-"), Some("STAJ")))
}

/// Örnek sonuçlar, tag'leri sahadaki gibi: `YTET_1003`, kalan senaryoda ayrıca `YGÖ-12`.
fn ornek_zip() -> Vec<u8> {
    let dir = sample_dir();
    let mut girdiler: Vec<(String, Vec<u8>)> = Vec::new();
    let mut adlar: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).collect();
    adlar.sort();
    for p in adlar {
        let ad = p.file_name().unwrap().to_string_lossy().to_string();
        let mut veri = std::fs::read(&p).unwrap();
        if ad.ends_with(".json") {
            // sahadaki gibi: gereksinim YTET_, Jira issue'su doğrudan proje anahtarıyla (@STAJ-42)
            let mut s = String::from_utf8(veri)
                .unwrap()
                .replace("\"value\":\"DOORS-", "\"value\":\"YTET_")
                .replace("\"value\":\"JIRA-STAJ-42\"", "\"value\":\"STAJ-42\"");
            if s.contains("\"name\":\"Geçerli bilgilerle giriş\"") {
                // geçen senaryoda Jira'da olmayan bir anahtar
                s = s.replacen(
                    "\"name\":\"tag\",\"value\":\"giris\"",
                    "\"name\":\"tag\",\"value\":\"giris\"},{\"name\":\"tag\",\"value\":\"STAJ-99\"",
                    1,
                );
            }
            if s.contains(&format!("\"name\":\"{KALAN}\"")) && s.contains("\"value\":\"sample-fail\"") {
                s = s.replace(
                    "\"name\":\"tag\",\"value\":\"sample-fail\"",
                    "\"name\":\"tag\",\"value\":\"YGÖ-12\"},{\"name\":\"tag\",\"value\":\"sample-fail\"",
                );
            }
            veri = s.into_bytes();
        }
        girdiler.push((format!("allure-results/{ad}"), veri));
    }
    let refs: Vec<(&str, &[u8])> = girdiler.iter().map(|(a, v)| (a.as_str(), v.as_slice())).collect();
    zip_entries(&refs)
}

async fn yukle(app: &TestApp) -> i64 {
    let (st, body) = app.ci_post(&format!("/api/ci/runs?{Q}"), "application/zip", ornek_zip()).await;
    assert_eq!(st, StatusCode::OK, "{body}");
    serde_json::from_str::<serde_json::Value>(&body).unwrap()["run_id"]
        .as_i64()
        .unwrap()
}

fn adet(html: &str) -> String {
    html.split("<h2>").nth(1).unwrap().split(' ').next().unwrap().to_string()
}

#[tokio::test]
async fn test_sw_6_4_ytet_ve_ygo_rapor_arama_csv_jira() {
    let app = uygulama();
    let id = yukle(&app).await;
    let run = run_json(&app, id).await;
    let sid = scenario_id(&run, KALAN);

    // SW-2.7: gereksinim kimliği YTET_ ile saklanır; YGÖ doors_ids'e girmez
    let (_, sayfa) = app.get(&format!("/scenarios/{sid}")).await;
    assert!(sayfa.contains(">YTET_1003<"), "YTET rozeti");
    assert!(sayfa.contains(">YGÖ-12<"), "YGÖ rozeti");
    assert!(
        !sayfa.contains(">@YTET_1003<") && !sayfa.contains(">@YGÖ-12<"),
        "düz tag olarak tekrar gösterilmez"
    );

    // SW-4.7: arama — numara, tam kimlik, ek kimlik (tam eşleşme)
    for (q, n) in [
        ("doors=1003", "1"),
        ("doors=YTET_1003", "1"),
        ("doors=YG%C3%96-12", "1"),
        ("doors=YG%C3%96-1", "0"),
        ("doors=DOORS-1003", "0"),
    ] {
        let (st, html) = app.get(&format!("/search?run={id}&{q}")).await;
        assert_eq!(st, StatusCode::OK);
        assert_eq!(adet(&html), n, "{q}");
    }
    let (_, arama) = app.get("/search").await;
    assert!(arama.contains("<label>YTET<input name=\"doors\""), "arama etiketi önek adıyla");

    // SW-4.8: alan seçmeli CSV — YTET başlığı, YGÖ ayrı sütun
    let (_, csv) = app
        .get(&format!("/export.csv?run={id}&status=kaldi&fields=senaryo,doors,ek:YG%C3%96-"))
        .await;
    let satirlar: Vec<&str> = csv.trim_start_matches('\u{feff}').lines().collect();
    assert_eq!(satirlar[0], "Senaryo,YTET,YGÖ");
    assert!(satirlar.contains(&format!("{KALAN},YTET_1003,YGÖ-12").as_str()), "{csv}");
    let (_, tumu) = app.get(&format!("/export.csv?run={id}")).await;
    let baslik = tumu.trim_start_matches('\u{feff}').lines().next().unwrap().to_string();
    assert!(baslik.contains(",YTET,YGÖ,") && !baslik.contains("DOORS"), "{baslik}");

    // SW-6.4: koşumun gereksinim CSV'si
    let (_, run_sayfa) = app.get(&format!("/runs/{id}")).await;
    assert!(run_sayfa.contains(">YTET CSV<") && !run_sayfa.contains("DOORS CSV"));
    let (st, dcsv) = app.get(&format!("/runs/{id}/doors.csv")).await;
    assert_eq!(st, StatusCode::OK);
    assert!(
        dcsv.starts_with("\u{feff}YTET,Senaryo,Feature,Sonuç,Jira,Koşum,Tarih,YGÖ\r\n"),
        "{dcsv}"
    );
    let satir = dcsv.lines().find(|l| l.starts_with("YTET_1003,")).unwrap();
    assert!(satir.ends_with(",YGÖ-12"), "{satir}");
    assert!(
        dcsv.lines().any(|l| l.starts_with("YTET_1001,") && l.ends_with(',')),
        "YGÖ'süz satırda boş hücre"
    );

    // SW-6.1: Jira bug'ında {{YTET}} ve {{YGÖ}} yalnızca metne yazılır, label olmaz
    let cfg = app.state.cfg.clone();
    let bug = app
        .state
        .db
        .read(move |c| {
            let sablon = divan::ayarlar::JiraSablon {
                ozet: "[{{YTET}}] {{senaryo}}".into(),
                aciklama: "Gereksinim: {{YTET}}\nBağlı: {{YGÖ}}\nEski: {{doors}}".into(),
                ..Default::default()
            };
            divan::jira::draft_for_scenario(c, &cfg, &sablon, sid)
        })
        .await
        .unwrap();
    assert_eq!(bug.summary, format!("[YTET_1003] {KALAN}"));
    assert_eq!(bug.description, "Gereksinim: YTET_1003\nBağlı: YGÖ-12\nEski: YTET_1003");
    assert!(bug.labels.is_empty());
    let (_, ayarlar) = app.get_admin("/admin/ayarlar").await;
    assert!(ayarlar.contains("{{YTET}}") && ayarlar.contains("{{YGÖ}}"), "yer tutucu listesi");
    assert!(!ayarlar.contains("doors_label"), "label kutusu kaldırıldı");
}

/// Sahte Jira: yalnızca issue okuma (STAJ-42 işlemde, STAJ-99 yok).
async fn sahte_jira() -> String {
    use axum::extract::Path;
    use axum::routing::get;
    async fn issue(Path(k): Path<String>) -> (StatusCode, axum::Json<serde_json::Value>) {
        match k.as_str() {
            "STAJ-42" => (
                StatusCode::OK,
                axum::Json(serde_json::json!({ "key": k, "fields": {
                    "summary": "Yanlış şifre mesajı ürün metniyle uyuşmuyor",
                    "status": { "name": "İşlemde", "statusCategory": { "key": "indeterminate" } },
                    "issuetype": { "name": "Bug" },
                    "assignee": { "displayName": "Ayşe Yılmaz" }
                } })),
            ),
            _ => (
                StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({ "errorMessages": ["Issue Does Not Exist"] })),
            ),
        }
    }
    spawn_fake(axum::Router::new().route("/rest/api/2/issue/{key}", get(issue))).await
}

#[tokio::test]
async fn test_sw_6_2_jira_tag_issue_bilgisi_raporda() {
    let url = sahte_jira().await;
    let app = app_with(move |c| {
        c.onekler = common::kimlik::Onekler::parse(Some("YTET_"), Some("YGÖ-"), Some("STAJ"));
        c.jira_url = Some(url);
        c.jira_token = Some(divan::config::Secret::new("jira-gizli-pat"));
        c.jira_project = Some("STAJ".into());
        c.jira_mode = divan::config::JiraMode::Off;
    });
    let id = yukle(&app).await;
    // koşum sonu işleri arka planda: Jira'dan çekilene kadar bekle
    let run = run_json(&app, id).await;
    let sid = scenario_id(&run, KALAN);
    let mut sayfa = String::new();
    for _ in 0..30 {
        settle().await;
        sayfa = app.get(&format!("/scenarios/{sid}")).await.1;
        if sayfa.contains("İşlemde") {
            break;
        }
    }
    assert!(
        sayfa.contains("Yanlış şifre mesajı ürün metniyle uyuşmuyor"),
        "başlık senaryo sayfasında"
    );
    assert!(sayfa.contains("STAJ-42 · İşlemde"), "rozette durum");
    assert!(sayfa.contains("Ayşe Yılmaz") && sayfa.contains("/browse/STAJ-42"));
    assert!(!sayfa.contains(">@STAJ-42<"), "düz tag olarak tekrar gösterilmez");
    let (_, rapor) = app.get(&format!("/runs/{id}")).await;
    assert!(rapor.contains("STAJ-42 · İşlemde"), "koşum sayfasında rozet");
    let i = rapor
        .find("STAJ-99")
        .map(|i| &rapor[i.saturating_sub(120)..(i + 80).min(rapor.len())])
        .unwrap_or("YOK");
    assert!(i.contains("bulunamadı"), "yok olan anahtar işaretli: {i}");
    // tag'le gelen anahtar kalan senaryoyu o issue'ya bağlar (mevcut akış)
    assert!(sayfa.contains(">tag<"), "kaynak: feature tag'i");
}

#[tokio::test]
async fn test_sw_6_2_bagli_jira_issuelari_raporda() {
    let url = sahte_jira().await;
    let jira = url.clone();
    let app = app_with(move |c| {
        c.onekler = common::kimlik::Onekler::parse(Some("YTET_"), Some("YGÖ-"), Some("STAJ"));
        c.jira_url = Some(url);
        c.jira_token = Some(divan::config::Secret::new("jira-gizli-pat"));
        c.jira_project = Some("STAJ".into());
        c.jira_mode = divan::config::JiraMode::Off;
    });
    let (st, body) = app
        .ci_post(&format!("/api/ci/runs?{Q}&app_version=2.0.1"), "application/zip", ornek_zip())
        .await;
    assert_eq!(st, StatusCode::OK, "{body}");
    let id = serde_json::from_str::<serde_json::Value>(&body).unwrap()["run_id"]
        .as_i64()
        .unwrap();
    let mut rapor = String::new();
    for _ in 0..30 {
        settle().await;
        rapor = app.get(&format!("/runs/{id}")).await.1;
        if rapor.contains("İşlemde") {
            break;
        }
    }
    // koşum sayfası: issue, başlık, durum, atanan, bağlı senaryo (linkli)
    let kart = &rapor[rapor.find("Jira issue'ları (").expect("Jira özeti kartı")..];
    let kart = &kart[..kart.find("</section>").unwrap()];
    assert!(kart.contains("STAJ-42") && kart.contains("Yanlış şifre mesajı ürün metniyle uyuşmuyor"));
    assert!(kart.contains("İşlemde") && kart.contains("Ayşe Yılmaz"));
    assert!(kart.contains(KALAN) && kart.contains("/browse/STAJ-42"));
    assert!(
        kart.contains("STAJ-99") && kart.contains("bulunamadı"),
        "Jira'da olmayan da listelenir"
    );
    assert!(kart.find("STAJ-42") < kart.find("STAJ-99"), "açık issue önce");
    // ekip linki: anahtar, başlık, durum var; Jira linki ve atanan yok
    let ck = app.admin_cookie();
    let paylas = |g: &'static str| {
        let (app, ck) = (&app, ck.clone());
        async move {
            let r = post_form(app, &format!("/admin/runs/{id}/share"), &ck, g).await;
            let loc = r.headers()["location"].to_str().unwrap().to_string();
            app.get(&format!("/s/{}", loc.split("share:").nth(1).unwrap())).await.1
        }
    };
    let ekip = paylas("").await;
    assert!(ekip.contains("Jira issue'ları") && ekip.contains("Yanlış şifre mesajı ürün metniyle uyuşmuyor"));
    assert!(ekip.contains("İşlemde") && !ekip.contains("Ayşe Yılmaz") && !ekip.contains("/browse/"));
    assert!(ekip.contains("Uygulama <b>2.0.1</b>"));
    // genel rapor: yalnızca görünen ürün hatasının issue'su
    let genel = paylas("gorunum=genel").await;
    assert!(genel.contains("STAJ-42") && genel.contains("Yanlış şifre mesajı ürün metniyle uyuşmuyor"));
    assert!(!genel.contains("STAJ-99"), "geçen senaryonun issue'su genel raporda yok");
    assert!(!genel.contains("/browse/") && genel.contains("Uygulama <b>2.0.1</b>"));
    // e-posta: sürüm ve Jira bölümü
    let mail = app
        .state
        .db
        .read(move |c| {
            let pid: i64 = c.query_row("SELECT project_id FROM runs WHERE id = ?1", [id], |r| r.get(0))?;
            divan::email::summary(c, "http://divan.test", Some(&jira), pid, "9001")
        })
        .await
        .unwrap()
        .unwrap();
    assert!(mail.subject.contains("uygulama 2.0.1"), "{}", mail.subject);
    assert!(mail.body.contains("Uygulama sürümü: 2.0.1") && mail.body.contains("uygulama 2.0.1,"));
    assert!(mail.body.contains("Jira issue'ları:"), "{}", mail.body);
    assert!(mail
        .body
        .contains("STAJ-42 [İşlemde] Yanlış şifre mesajı ürün metniyle uyuşmuyor — 1 senaryo"));
    assert!(mail.body.contains("/browse/STAJ-42"));
}
