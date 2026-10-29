//! SW-9.5: admin girişi.

use crate::auth::{self, Admin};
use crate::web::render;
use crate::AppState;
use askama::Template;
use axum::extract::{Query, Request, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Form;
use serde::Deserialize;

#[derive(Template)]
#[template(path = "login.html")]
struct LoginPage {
    l: crate::web::pages::Layout,
    next: String,
    error: Option<String>,
}

fn lay() -> crate::web::pages::Layout {
    crate::web::pages::Layout { admin: false, banner: None, share: None }
}

#[derive(Deserialize)]
pub struct NextQuery {
    next: Option<String>,
}

fn safe_next(n: Option<String>) -> String {
    n.filter(|n| n.starts_with('/') && !n.starts_with("//")).unwrap_or_else(|| "/".into())
}

pub async fn login_page(Query(q): Query<NextQuery>) -> Response {
    render(&LoginPage { next: safe_next(q.next), error: None, l: lay() })
}

#[derive(Deserialize)]
pub struct LoginForm {
    password: String,
    next: Option<String>,
}

pub async fn login(State(st): State<AppState>, req: Request) -> Response {
    let ip = auth::client_ip(req.extensions());
    let Ok(Form(form)) = Form::<LoginForm>::from_request(req, &st).await else {
        return (StatusCode::BAD_REQUEST, "Geçersiz form").into_response();
    };
    let next = safe_next(form.next);
    if let Err(wait) = st.throttle.check(ip) {
        tracing::warn!(ip = %ip, "admin girişi yavaşlatıldı");
        let page = LoginPage { next, error: Some(format!("Çok fazla hatalı deneme. {} sn sonra tekrar deneyin.", wait.as_secs() + 1)), l: lay() };
        return (StatusCode::TOO_MANY_REQUESTS, render(&page)).into_response();
    }
    let auth = st.auth.clone();
    let ok = tokio::task::spawn_blocking(move || auth.verify("admin", &form.password)).await.unwrap_or(false);
    if ok {
        st.throttle.success(ip);
        tracing::info!(ip = %ip, "admin girişi başarılı");
        let token = st.sessions.create();
        ([(header::SET_COOKIE, auth::session_cookie(&token))], Redirect::to(&next)).into_response()
    } else {
        st.throttle.failure(ip);
        tracing::warn!(ip = %ip, "admin girişi başarısız");
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        let page = LoginPage { next, error: Some("Şifre yanlış".into()), l: lay() };
        (StatusCode::UNAUTHORIZED, render(&page)).into_response()
    }
}

pub async fn logout(State(st): State<AppState>, headers: axum::http::HeaderMap) -> Response {
    if let Some(t) = auth::cookie_value(&headers, auth::SESSION_COOKIE) {
        st.sessions.remove(&t);
    }
    ([(header::SET_COOKIE, auth::clear_cookie())], Redirect::to("/")).into_response()
}

pub async fn me(_a: Admin) -> &'static str {
    "admin"
}

use axum::extract::FromRequest;
