//! SW-10.9: stajci ikilisi sunucudan dağıtılır; CI şablonu her koşumda buradan indirir.

use crate::AppState;
use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct OsQuery {
    os: Option<String>,
}

pub fn binary_name(os: &str) -> Option<&'static str> {
    match os {
        "linux" => Some("stajci-linux"),
        "windows" => Some("stajci-windows.exe"),
        _ => None,
    }
}

fn candidates(st: &AppState, os: &str) -> Vec<PathBuf> {
    let name = binary_name(os).unwrap_or("stajci");
    let mut v = Vec::new();
    if let Some(d) = &st.cfg.stajci_dir {
        v.push(d.join(name));
    }
    v.push(st.paths.tools_dir().join(name));
    // sunucu ikilisinin yanında duran stajci (aynı işletim sistemi için)
    let same_os = (os == "windows") == cfg!(windows);
    if same_os {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                v.push(dir.join(if cfg!(windows) { "stajci.exe" } else { "stajci" }));
            }
        }
    }
    v
}

pub async fn stajci(State(st): State<AppState>, Query(q): Query<OsQuery>) -> Response {
    let os =
        q.os.unwrap_or_else(|| if cfg!(windows) { "windows".into() } else { "linux".into() });
    let Some(name) = binary_name(&os) else {
        return (StatusCode::BAD_REQUEST, "os linux ya da windows olmalı").into_response();
    };
    for p in candidates(&st, &os) {
        if p.is_file() {
            return match tokio::fs::File::open(&p).await {
                Ok(f) => {
                    let len = f.metadata().await.map(|m| m.len()).unwrap_or(0);
                    let body = axum::body::Body::from_stream(tokio_util::io::ReaderStream::new(f));
                    (
                        [
                            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
                            (header::CONTENT_LENGTH, len.to_string()),
                            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{name}\"")),
                            (header::HeaderName::from_static(common::VERSION_HEADER), common::VERSION.to_string()),
                        ],
                        body,
                    )
                        .into_response()
                }
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            };
        }
    }
    tracing::warn!(os = %os, "stajci ikilisi bulunamadı");
    (StatusCode::NOT_FOUND, format!("{name} bulunamadı; DATA_DIR/tools altına koyun")).into_response()
}
