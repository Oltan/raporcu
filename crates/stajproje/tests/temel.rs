//! Faz 1 entegrasyon testleri: SW-10.5, SW-9.1, SW-9.5, SW-10.7
mod util;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use util::*;

#[tokio::test]
async fn test_sw_10_5_health_returns_200() {
    let app = app();
    let (st, body) = app.get("/api/health").await;
    assert_eq!(st, StatusCode::OK);
    assert!(body.contains("\"durum\":\"ok\""));
}

#[tokio::test]
async fn test_sw_10_5_static_files_embedded() {
    let app = app();
    let (st, body) = app.get("/static/htmx.min.js").await;
    assert_eq!(st, StatusCode::OK);
    assert!(body.len() > 10_000);
}

#[tokio::test]
async fn test_sw_9_1_ci_endpoints_require_token() {
    let app = app();
    let r = app.send(Request::get("/api/ci/ping").body(Body::empty()).unwrap()).await;
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    let r = app
        .send(Request::get("/api/ci/ping").header("authorization", "Bearer yanlis").body(Body::empty()).unwrap())
        .await;
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    let r = app
        .send(Request::get("/api/ci/ping").header("authorization", format!("Bearer {CI_TOKEN}")).body(Body::empty()).unwrap())
        .await;
    assert_eq!(r.status(), StatusCode::OK);
    let r = app.send(Request::get("/api/ci/ping").header("x-ci-token", CI_TOKEN).body(Body::empty()).unwrap()).await;
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_sw_9_1_no_token_configured_rejects_all() {
    let app = app_with(|c| c.ci_api_token = None);
    let r = app
        .send(Request::get("/api/ci/ping").header("authorization", "Bearer ").body(Body::empty()).unwrap())
        .await;
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_sw_10_9_incompatible_stajci_version_rejected() {
    let app = app();
    let r = app
        .send(
            Request::get("/api/ci/ping")
                .header("authorization", format!("Bearer {CI_TOKEN}"))
                .header("x-stajci-version", "99.0.0")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(r.status(), StatusCode::CONFLICT);
    assert!(body_string(r).await.contains("stajci sürümü uyumsuz"));
    let r = app
        .send(
            Request::get("/api/ci/ping")
                .header("authorization", format!("Bearer {CI_TOKEN}"))
                .header("x-stajci-version", common::VERSION)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(r.status(), StatusCode::OK);
}

async fn login(app: &TestApp, pw: &str, ip: [u8; 4]) -> axum::http::Response<Body> {
    let mut req = Request::post("/login")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(format!("password={}&next=%2Fadmin", urlencoding::encode(pw))))
        .unwrap();
    req.extensions_mut().insert(axum::extract::ConnectInfo(std::net::SocketAddr::from((ip, 5000))));
    app.send(req).await
}

#[tokio::test]
async fn test_sw_9_5_wrong_password_cannot_do_admin_ops() {
    let hash = stajproje::auth::hash_password("Dogru-Sifre-123").unwrap();
    let app = app_with(|c| c.admin_password_hash = Some(stajproje::config::Secret::new(hash)));

    // giriş yok → admin uç reddedilir
    let (st, _) = app.get("/api/admin/me").await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);

    // yanlış şifre → çerez yok, 401
    let r = login(&app, "YanlisSifre-XYZ", [10, 1, 1, 1]).await;
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    assert!(r.headers().get("set-cookie").is_none());
    // hemen ikinci deneme yavaşlatılır
    let r = login(&app, "Dogru-Sifre-123", [10, 1, 1, 1]).await;
    assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);

    // başka IP'den doğru şifre → çerez, admin uç açılır
    let r = login(&app, "Dogru-Sifre-123", [10, 1, 1, 2]).await;
    assert_eq!(r.status(), StatusCode::SEE_OTHER);
    let cookie = r.headers().get("set-cookie").unwrap().to_str().unwrap().to_string();
    assert!(cookie.contains("HttpOnly") && cookie.contains("SameSite=Strict"));
    let kv = cookie.split(';').next().unwrap().to_string();
    let r = app.send(Request::get("/api/admin/me").header("cookie", kv).body(Body::empty()).unwrap()).await;
    assert_eq!(r.status(), StatusCode::OK);

    // SW-9.5 / SW-10.7: şifre loglarda yok, başarısız giriş logu yalnızca IP içerir
    let all = stajproje::logging::test_capture().lock().unwrap().join("\n");
    assert!(!all.contains("YanlisSifre-XYZ") && !all.contains("Dogru-Sifre-123"));
    let failed = log_lines("admin girişi başarısız");
    assert!(failed.iter().any(|l| l["ip"] == "10.1.1.1"));
    assert!(!log_lines("admin girişi başarılı").is_empty());
}

#[tokio::test]
async fn test_sw_9_5_admin_page_redirects_to_login() {
    let app = app();
    let r = app.send(Request::get("/api/admin/me").header("accept", "text/html").body(Body::empty()).unwrap()).await;
    assert_eq!(r.status(), StatusCode::SEE_OTHER);
    assert!(r.headers()["location"].to_str().unwrap().starts_with("/login"));
}

#[tokio::test]
async fn test_sw_10_9_stajci_served_per_os() {
    let app = app();
    std::fs::write(app.state.paths.tools_dir().join("stajci-linux"), b"ELF-sahte").unwrap();
    std::fs::write(app.state.paths.tools_dir().join("stajci-windows.exe"), b"MZ-sahte").unwrap();
    let r = app.send(Request::get("/api/tools/stajci?os=linux").body(Body::empty()).unwrap()).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(r.headers()["x-stajci-version"], common::VERSION);
    assert_eq!(body_string(r).await, "ELF-sahte");
    let (st, body) = app.get("/api/tools/stajci?os=windows").await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(body, "MZ-sahte");
    let (st, _) = app.get("/api/tools/stajci?os=mac").await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_sw_3_5_stalled_run_becomes_interrupted() {
    let app = app();
    let (_, v) = app
        .ci_json("/api/ci/runs/start", serde_json::json!({ "project_path": "ekip/yarida", "branch": "main", "total": 5 }))
        .await;
    let id = v["run_id"].as_i64().unwrap();
    // zaman aşımı 0: hemen interrupted
    let ids = app.state.db.write(|c| stajproje::stall::mark_stalled(c, std::time::Duration::ZERO)).await.unwrap();
    assert!(ids.contains(&id));
    let (_, run) = app.get(&format!("/api/runs/{id}")).await;
    assert!(run.contains("\"status\":\"interrupted\""));
    assert!(log_lines("koşum interrupted oldu").iter().any(|l| l["run_id"] == id));
    // süresi dolmamış koşuma dokunulmaz
    let (_, v) = app
        .ci_json("/api/ci/runs/start", serde_json::json!({ "project_path": "ekip/yarida", "branch": "main" }))
        .await;
    let id2 = v["run_id"].as_i64().unwrap();
    let ids = app.state.db.write(|c| stajproje::stall::mark_stalled(c, std::time::Duration::from_secs(600))).await.unwrap();
    assert!(!ids.contains(&id2));
}

#[tokio::test]
async fn test_sw_3_5_finish_endpoint_marks_interrupted() {
    let app = app();
    let (_, v) = app.ci_json("/api/ci/runs/start", serde_json::json!({ "project_path": "ekip/f", "branch": "main" })).await;
    let id = v["run_id"].as_i64().unwrap();
    let (st, v) = app.ci_json(&format!("/api/ci/runs/{id}/finish"), serde_json::json!({ "status": "interrupted" })).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["status"], "interrupted");
    let (st, _) = app.ci_json(&format!("/api/ci/runs/{id}/finish"), serde_json::json!({ "status": "baska" })).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
}
