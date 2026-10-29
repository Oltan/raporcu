#![allow(dead_code)]
//! Entegrasyon testleri için ortak yardımcılar: geçici DATA_DIR ile uygulama.

use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use http_body_util::BodyExt;
use stajproje::{AppState, Config};
use tower::ServiceExt;

pub const CI_TOKEN: &str = "test-ci-token";

pub struct TestApp {
    pub state: AppState,
    pub router: axum::Router,
    pub dir: tempfile::TempDir,
}

pub fn app() -> TestApp {
    app_with(|_| {})
}

pub fn app_with(f: impl FnOnce(&mut Config)) -> TestApp {
    stajproje::logging::test_capture();
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = Config::for_tests(dir.path().join("data"));
    f(&mut cfg);
    let state = AppState::new(cfg, stajproje::logging::test_recent()).unwrap();
    let router = stajproje::router(state.clone());
    TestApp { state, router, dir }
}

impl TestApp {
    pub async fn send(&self, req: Request<Body>) -> Response<Body> {
        self.router.clone().oneshot(req).await.unwrap()
    }

    pub async fn get(&self, uri: &str) -> (StatusCode, String) {
        let r = self.send(Request::get(uri).body(Body::empty()).unwrap()).await;
        let st = r.status();
        (st, body_string(r).await)
    }

    pub async fn get_admin(&self, uri: &str) -> (StatusCode, String) {
        let token = self.state.sessions.create();
        let r = self
            .send(
                Request::get(uri)
                    .header("cookie", format!("staj_oturum={token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
        let st = r.status();
        (st, body_string(r).await)
    }

    pub fn admin_cookie(&self) -> String {
        format!("staj_oturum={}", self.state.sessions.create())
    }

    pub async fn ci_post(&self, uri: &str, content_type: &str, body: impl Into<Body>) -> (StatusCode, String) {
        let r = self
            .send(
                Request::post(uri)
                    .header("authorization", format!("Bearer {CI_TOKEN}"))
                    .header("content-type", content_type)
                    .body(body.into())
                    .unwrap(),
            )
            .await;
        let st = r.status();
        (st, body_string(r).await)
    }

    pub async fn ci_json(&self, uri: &str, v: serde_json::Value) -> (StatusCode, serde_json::Value) {
        let (st, s) = self.ci_post(uri, "application/json", v.to_string()).await;
        (st, serde_json::from_str(&s).unwrap_or(serde_json::Value::String(s)))
    }
}

pub async fn body_string(r: Response<Body>) -> String {
    let b = r.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8_lossy(&b).to_string()
}

pub fn sample_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/allure/ornek-giris")
}

/// Bir klasörü (allure-results) zip'e çevirir.
pub fn zip_dir(dir: &std::path::Path) -> Vec<u8> {
    zip_dir_filtered(dir, |_| true)
}

pub fn zip_dir_filtered(dir: &std::path::Path, keep: impl Fn(&str) -> bool) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut w = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default();
        let mut names: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
        names.sort();
        for p in names {
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            if !keep(&name) {
                continue;
            }
            w.start_file(format!("allure-results/{name}"), opts).unwrap();
            std::io::Write::write_all(&mut w, &std::fs::read(&p).unwrap()).unwrap();
        }
        w.finish().unwrap();
    }
    buf.into_inner()
}

pub fn zip_entries(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut w = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default();
        for (n, d) in entries {
            w.start_file(*n, opts).unwrap();
            std::io::Write::write_all(&mut w, d).unwrap();
        }
        w.finish().unwrap();
    }
    buf.into_inner()
}

pub fn log_lines(needle: &str) -> Vec<serde_json::Value> {
    stajproje::logging::captured_lines(&stajproje::logging::test_capture(), needle)
}

/// Örnek zip'i yükler, koşum id'sini döner.
pub async fn upload_sample(app: &TestApp, query: &str) -> i64 {
    let (st, body) = app
        .ci_post(&format!("/api/ci/runs?{query}"), "application/zip", zip_dir(&sample_dir()))
        .await;
    assert_eq!(st, StatusCode::OK, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    v["run_id"].as_i64().unwrap()
}

pub async fn run_json(app: &TestApp, id: i64) -> serde_json::Value {
    let (_, b) = app.get(&format!("/api/runs/{id}")).await;
    serde_json::from_str(&b).unwrap()
}

pub fn scenario_id(run: &serde_json::Value, name: &str) -> i64 {
    run["senaryolar"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["senaryo"]["name"] == name)
        .unwrap()["senaryo"]["id"]
        .as_i64()
        .unwrap()
}

/// Test içinde sahte HTTP sunucusu (Jira, GitLab). Adresini döner.
pub async fn spawn_fake(router: axum::Router) -> String {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(l, router).await.unwrap() });
    format!("http://{addr}")
}

pub async fn post_form(app: &TestApp, uri: &str, cookie: &str, form: &str) -> Response<Body> {
    app.send(
        Request::post(uri)
            .header("cookie", cookie)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(form.to_string()))
            .unwrap(),
    )
    .await
}

/// Koşum bittikten sonra arka planda çalışan işlerin (Jira) bitmesini bekler.
pub async fn settle() {
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
}
