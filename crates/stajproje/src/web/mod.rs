//! HTTP uçları ve sayfalar.

pub mod admin;

use crate::AppState;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/login", get(admin::login_page).post(admin::login))
        .route("/logout", post(admin::logout))
        .route("/api/admin/me", get(admin::me))
        .route("/static/{*path}", get(static_file))
}

#[derive(rust_embed::RustEmbed)]
#[folder = "static/"]
struct Static;

/// Statik dosyalar ikiliye gömülüdür (kapalı ağda CDN yok).
pub async fn static_file(axum::extract::Path(path): axum::extract::Path<String>) -> Response {
    match Static::get(&path) {
        Some(f) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            (
                [
                    (axum::http::header::CONTENT_TYPE, mime.to_string()),
                    (axum::http::header::CACHE_CONTROL, "public, max-age=3600".to_string()),
                ],
                f.data.into_owned(),
            )
                .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "durum": "ok", "surum": common::VERSION }))
}

/// Uygulama hatası: kullanıcıya kısa mesaj, loga ayrıntı.
pub struct AppError(pub StatusCode, pub String);

impl AppError {
    pub fn bad(msg: impl Into<String>) -> Self {
        AppError(StatusCode::BAD_REQUEST, msg.into())
    }
    pub fn not_found() -> Self {
        AppError(StatusCode::NOT_FOUND, "Bulunamadı".into())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        tracing::error!(hata = %e, "istek işlenemedi");
        AppError(StatusCode::INTERNAL_SERVER_ERROR, "Sunucu hatası".into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.0, self.1).into_response()
    }
}

pub fn render<T: askama::Template>(t: &T) -> Response {
    match t.render() {
        Ok(s) => Html(s).into_response(),
        Err(e) => {
            tracing::error!(hata = %e, "şablon işlenemedi");
            (StatusCode::INTERNAL_SERVER_ERROR, "Şablon hatası").into_response()
        }
    }
}
