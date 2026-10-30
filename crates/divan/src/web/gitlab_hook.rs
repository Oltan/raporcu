//! POST /api/gitlab/webhook — X-Gitlab-Token ile doğrulanır (SW-9.3).

use crate::AppState;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use subtle::ConstantTimeEq;

pub async fn webhook(State(st): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    let given = headers.get("x-gitlab-token").and_then(|v| v.to_str().ok()).unwrap_or("");
    let ok = st
        .cfg
        .gitlab_webhook_token
        .as_ref()
        .is_some_and(|t| t.expose().len() == given.len() && bool::from(t.expose().as_bytes().ct_eq(given.as_bytes())));
    if !ok {
        tracing::warn!("webhook reddedildi: X-Gitlab-Token yanlış");
        return (StatusCode::UNAUTHORIZED, "X-Gitlab-Token yanlış").into_response();
    }
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&body) else {
        return (StatusCode::BAD_REQUEST, "json değil").into_response();
    };
    let kind = v["object_kind"].as_str().unwrap_or("").to_string();
    let st2 = st.clone();
    let res = st
        .db
        .write(move |c| {
            match kind.as_str() {
                "pipeline" => {
                    if let Some((pid, pipeline, status, source)) = crate::gitlab::apply_pipeline_event(c, &v)? {
                        // SW-5.3: gece (schedule) pipeline'ı bitince özet e-posta
                        let done = matches!(status.as_str(), "success" | "failed");
                        if done && source.as_deref() == Some("schedule") {
                            if let Err(e) = crate::email::send_for_pipeline(&st2, c, pid, &pipeline) {
                                tracing::warn!(hata = %e, "özet e-posta gönderilemedi");
                            }
                        }
                    }
                }
                "merge_request" => {
                    crate::gitlab::apply_mr_event(c, &v)?;
                }
                _ => {}
            }
            Ok(())
        })
        .await;
    match res {
        Ok(()) => (StatusCode::OK, "ok").into_response(),
        Err(e) => crate::web::AppError::from(e).into_response(),
    }
}
