//! SW-4.1, SW-10.6: ek dosyaları HTTP range desteğiyle, parça parça sunulur.

use crate::queries;
use crate::AppState;
use axum::body::Body;
use axum::extract::{Path, Request, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use tower::ServiceExt;
use tower_http::services::ServeFile;

pub async fn serve_attachment_file(st: &AppState, att: &queries::AttachmentRow, req: Request) -> Response {
    if let Some(reason) = &att.deleted_reason {
        return (StatusCode::GONE, reason.clone()).into_response();
    }
    let path = st.paths.attachment(att.run_id, &att.source);
    if !path.is_file() {
        return (StatusCode::NOT_FOUND, "Ek dosyası henüz gelmedi ya da yok").into_response();
    }
    let mime = att.mime.clone().unwrap_or_else(|| "application/octet-stream".into());
    match ServeFile::new_with_mime(&path, &mime.parse().unwrap_or(mime_guess::mime::APPLICATION_OCTET_STREAM)).oneshot(req).await {
        Ok(mut r) => {
            r.headers_mut().insert(header::CACHE_CONTROL, "private, max-age=86400".parse().unwrap());
            r.map(Body::new).into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub async fn attachment(State(st): State<AppState>, Path(id): Path<i64>, req: Request) -> Response {
    let att = match st.db.read(move |c| queries::attachment(c, id)).await {
        Ok(Some(a)) => a,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => return crate::web::AppError::from(e).into_response(),
    };
    serve_attachment_file(&st, &att, req).await
}
