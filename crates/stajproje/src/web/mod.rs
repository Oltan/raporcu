//! HTTP uçları ve sayfalar.

pub mod admin;
pub mod ci;
pub mod files;
pub mod tools;

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
        .route("/files/{id}", get(files::attachment))
        .route("/api/tools/stajci", get(tools::stajci))
        .route("/api/runs", get(api::runs))
        .route("/api/runs/{id}", get(api::run))
        .route("/api/scenarios/{id}", get(api::scenario))
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

pub mod api {
    //! Salt okunur JSON API (giriş gerektirmez; kapalı ağ).
    use super::AppError;
    use crate::queries;
    use crate::AppState;
    use axum::extract::{Path, Query, State};
    use axum::Json;
    use serde::Deserialize;
    use serde_json::{json, Value};

    #[derive(Deserialize, Default)]
    pub struct RunListQuery {
        pub project: Option<i64>,
        pub branch: Option<String>,
        pub page: Option<i64>,
    }

    pub async fn runs(State(st): State<AppState>, Query(q): Query<RunListQuery>) -> Result<Json<Value>, AppError> {
        let page = q.page.unwrap_or(1).max(1);
        let v = st
            .db
            .read(move |c| {
                let runs = queries::list_runs(c, q.project, q.branch.as_deref(), 50, (page - 1) * 50)?;
                Ok(json!({ "sayfa": page, "kosumlar": runs }))
            })
            .await?;
        Ok(Json(v))
    }

    pub async fn run(State(st): State<AppState>, Path(id): Path<i64>) -> Result<Json<Value>, AppError> {
        let v = st
            .db
            .read(move |c| {
                let Some(run) = queries::run(c, id)? else { return Ok(None) };
                let mut list = Vec::new();
                for s in queries::run_scenarios(c, id)? {
                    let atts = queries::attachments(c, s.id)?;
                    let attempts = queries::attempts(c, id, &s.scenario_key)?.len();
                    list.push(json!({ "senaryo": s, "ekler": atts, "deneme_sayisi": attempts }));
                }
                Ok(Some(json!({ "kosum": run, "senaryolar": list })))
            })
            .await?
            .ok_or_else(AppError::not_found)?;
        Ok(Json(v))
    }

    pub async fn scenario(State(st): State<AppState>, Path(id): Path<i64>) -> Result<Json<Value>, AppError> {
        let v = st
            .db
            .read(move |c| {
                let Some(s) = queries::scenario(c, id)? else { return Ok(None) };
                Ok(Some(json!({
                    "senaryo": s,
                    "adimlar": queries::steps(c, id)?,
                    "ekler": queries::attachments(c, id)?,
                    "denemeler": queries::attempts(c, s.run_id, &s.scenario_key)?,
                })))
            })
            .await?
            .ok_or_else(AppError::not_found)?;
        Ok(Json(v))
    }
}
