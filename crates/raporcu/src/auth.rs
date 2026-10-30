//! SW-9.1 CI token koruması, SW-9.5 admin girişi, SW-10.9 stajci sürüm kontrolü.

use crate::AppState;
use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::{ConnectInfo, FromRequestParts, Request, State};
use axum::http::{header, request::Parts, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use subtle::ConstantTimeEq;

pub const SESSION_COOKIE: &str = "staj_oturum";
const SESSION_TTL: Duration = Duration::from_secs(12 * 3600);

/// Giriş kontrolü tek arayüz arkasında: ileride AD/LDAP buraya ikinci bir uygulama olarak eklenir.
pub trait Authenticator: Send + Sync {
    fn verify(&self, username: &str, password: &str) -> bool;
}

/// Şimdilik tek kullanıcı: `admin`, şifre hash'i `.env` içinde `ADMIN_PASSWORD_HASH` (argon2).
pub struct LocalAdmin {
    hash: Option<String>,
}

impl LocalAdmin {
    pub fn new(hash: Option<String>) -> Self {
        LocalAdmin { hash }
    }
}

impl Authenticator for LocalAdmin {
    fn verify(&self, username: &str, password: &str) -> bool {
        let Some(hash) = &self.hash else { return false };
        if username != "admin" {
            return false;
        }
        let Ok(parsed) = PasswordHash::new(hash) else { return false };
        Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok()
    }
}

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| anyhow::anyhow!("hash üretilemedi: {e}"))
}

/// `.env` dosyasında bir anahtarı ekler ya da değiştirir. Diğer satırlar korunur.
pub fn update_env_file(path: &Path, key: &str, value: &str) -> anyhow::Result<()> {
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    // argon2 hash'inde `$` var; dotenvy tek tırnak içini olduğu gibi okur.
    let line = format!("{key}='{value}'");
    let mut found = false;
    let mut out: Vec<String> = existing
        .lines()
        .map(|l| {
            if l.trim_start().starts_with(&format!("{key}=")) {
                found = true;
                line.clone()
            } else {
                l.to_string()
            }
        })
        .collect();
    if !found {
        out.push(line);
    }
    std::fs::write(path, out.join("\n") + "\n")?;
    Ok(())
}

pub fn random_token() -> String {
    format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple())
}

#[derive(Clone, Default)]
pub struct Sessions(Arc<Mutex<HashMap<String, Instant>>>);

impl Sessions {
    pub fn create(&self) -> String {
        let t = random_token();
        let mut m = self.0.lock().unwrap();
        m.retain(|_, exp| *exp > Instant::now());
        m.insert(t.clone(), Instant::now() + SESSION_TTL);
        t
    }
    pub fn valid(&self, token: &str) -> bool {
        self.0.lock().unwrap().get(token).is_some_and(|exp| *exp > Instant::now())
    }
    pub fn remove(&self, token: &str) {
        self.0.lock().unwrap().remove(token);
    }
}

/// Başarısız denemeleri yavaşlatır: her hatadan sonra bekleme süresi ikiye katlanır (en fazla 60 sn).
#[derive(Clone, Default)]
pub struct LoginThrottle(Arc<Mutex<HashMap<IpAddr, (u32, Instant)>>>);

impl LoginThrottle {
    /// Şimdi deneme yapılabilir mi; yapılamıyorsa kalan süre.
    pub fn check(&self, ip: IpAddr) -> Result<(), Duration> {
        let m = self.0.lock().unwrap();
        match m.get(&ip) {
            Some((_, until)) if *until > Instant::now() => Err(*until - Instant::now()),
            _ => Ok(()),
        }
    }
    pub fn failure(&self, ip: IpAddr) {
        let mut m = self.0.lock().unwrap();
        let e = m.entry(ip).or_insert((0, Instant::now()));
        e.0 += 1;
        let secs = 1u64 << (e.0 - 1).min(6);
        e.1 = Instant::now() + Duration::from_secs(secs.min(60));
    }
    pub fn success(&self, ip: IpAddr) {
        self.0.lock().unwrap().remove(&ip);
    }
}

pub fn client_ip(parts_ext: &axum::http::Extensions) -> IpAddr {
    parts_ext
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip())
        .unwrap_or(IpAddr::from([127, 0, 0, 1]))
}

pub fn cookie_value(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|kv| kv.trim().split_once('='))
        .find(|(k, _)| *k == name)
        .map(|(_, v)| v.to_string())
}

pub fn session_cookie(token: &str) -> String {
    format!(
        "{SESSION_COOKIE}={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age={}",
        SESSION_TTL.as_secs()
    )
}

pub fn clear_cookie() -> String {
    format!("{SESSION_COOKIE}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0")
}

/// Admin işlemi gerektiren uçların çıkarıcısı.
pub struct Admin;

pub enum AdminRejection {
    Redirect(String),
    Unauthorized,
}

impl IntoResponse for AdminRejection {
    fn into_response(self) -> Response {
        match self {
            AdminRejection::Redirect(next) => Redirect::to(&format!("/login?next={}", urlencoding::encode(&next))).into_response(),
            AdminRejection::Unauthorized => (StatusCode::UNAUTHORIZED, "Admin girişi gerekli").into_response(),
        }
    }
}

impl FromRequestParts<AppState> for Admin {
    type Rejection = AdminRejection;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        if let Some(t) = cookie_value(&parts.headers, SESSION_COOKIE) {
            if state.sessions.valid(&t) {
                return Ok(Admin);
            }
        }
        let wants_html = parts
            .headers
            .get(header::ACCEPT)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.contains("text/html"));
        if parts.method == Method::GET && wants_html {
            Err(AdminRejection::Redirect(parts.uri.path().to_string()))
        } else {
            Err(AdminRejection::Unauthorized)
        }
    }
}

fn ct_eq(a: &str, b: &str) -> bool {
    a.len() == b.len() && bool::from(a.as_bytes().ct_eq(b.as_bytes()))
}

/// SW-9.1 + SW-10.9: `/api/ci/*` uçları için ara katman.
pub async fn ci_guard(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let headers = req.headers();
    let given = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .or_else(|| headers.get("x-ci-token").and_then(|v| v.to_str().ok()))
        .map(str::trim);
    let ok = match (&state.cfg.ci_api_token, given) {
        (Some(expected), Some(g)) => ct_eq(expected.expose(), g),
        _ => false,
    };
    if !ok {
        tracing::warn!(yol = %req.uri().path(), "CI isteği reddedildi: CI_API_TOKEN yok ya da yanlış");
        return (StatusCode::UNAUTHORIZED, "CI_API_TOKEN eksik ya da yanlış").into_response();
    }
    if let Some(v) = headers.get(common::VERSION_HEADER).and_then(|v| v.to_str().ok()) {
        if !common::versions_compatible(v, common::VERSION) {
            tracing::warn!(stajci_surum = v, sunucu_surum = common::VERSION, "stajci sürüm uyuşmazlığı");
            return (
                StatusCode::CONFLICT,
                format!(
                    "stajci sürümü uyumsuz: stajci {v}, sunucu {}. Yeni stajci'yi {}/api/tools/stajci adresinden indirin.",
                    common::VERSION,
                    state.cfg.public_url
                ),
            )
                .into_response();
        }
    }
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_9_5_argon2_verify() {
        let h = hash_password("dogru-sifre").unwrap();
        assert!(h.starts_with("$argon2"));
        let a = LocalAdmin::new(Some(h));
        assert!(a.verify("admin", "dogru-sifre"));
        assert!(!a.verify("admin", "yanlis"));
        assert!(!a.verify("baska", "dogru-sifre"));
        assert!(!LocalAdmin::new(None).verify("admin", ""));
    }

    #[test]
    fn test_sw_9_5_env_file_update_keeps_other_lines() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join(".env");
        std::fs::write(&p, "DATA_DIR=veri\nADMIN_PASSWORD_HASH='eski'\n").unwrap();
        update_env_file(&p, "ADMIN_PASSWORD_HASH", "$argon2id$v=19$yeni").unwrap();
        let s = std::fs::read_to_string(&p).unwrap();
        assert!(s.contains("DATA_DIR=veri"));
        assert!(s.contains("ADMIN_PASSWORD_HASH='$argon2id$v=19$yeni'"));
        assert!(!s.contains("eski"));
        // dotenvy ile geri okunabilir
        let parsed: Vec<(String, String)> = dotenvy::from_path_iter(&p).unwrap().map(|r| r.unwrap()).collect();
        assert!(parsed.contains(&("ADMIN_PASSWORD_HASH".into(), "$argon2id$v=19$yeni".into())));
    }

    #[test]
    fn test_sw_9_5_throttle_grows() {
        let t = LoginThrottle::default();
        let ip = IpAddr::from([10, 0, 0, 1]);
        assert!(t.check(ip).is_ok());
        t.failure(ip);
        assert!(t.check(ip).is_err());
        t.success(ip);
        assert!(t.check(ip).is_ok());
    }
}
