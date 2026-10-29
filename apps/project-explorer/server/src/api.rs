//! HTTP endpoints for server-owned app launch and viewer sessions.
use crate::{
    lifecycle::{Status, Token},
    manager::Error,
    runtime::Service,
};
use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::PathBuf;
use tower_http::services::ServeDir;

#[derive(Clone)]
struct ApiState {
    service: Service,
    origin: String,
    heartbeat_seconds: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyRequest {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HeartbeatRequest {
    session: String,
}

/// Build a router serving static gallery files and same-origin hosting endpoints.
/// `origin` is the exact browser-visible origin; Caddy must forward requests to
/// this server over a trusted connection. POST bodies have fixed schemas.
pub fn router(service: Service, origin: String, assets: PathBuf, heartbeat_seconds: u64) -> Router {
    let state = ApiState {
        service,
        origin,
        heartbeat_seconds,
    };
    Router::new()
        .route("/internal/authorize", get(authorize_upstream))
        .route("/running/{id}/", get(viewer))
        .route("/running/{id}/{generation}/", get(viewer))
        .route("/api/hosting/apps", get(apps))
        .route("/api/hosting/apps/{id}", get(status))
        .route("/api/hosting/apps/{id}/launch", post(launch))
        .route(
            "/api/hosting/apps/{id}/{generation}/heartbeat",
            post(heartbeat),
        )
        .route(
            "/api/hosting/apps/{id}/{generation}/results",
            get(recording),
        )
        .route(
            "/api/hosting/apps/{id}/{generation}/results/list",
            get(result_list),
        )
        .route(
            "/api/hosting/apps/{id}/{generation}/results/captures/{name}",
            get(capture),
        )
        .route(
            "/api/hosting/apps/{id}/{generation}/results/files/{name}",
            get(additional_file),
        )
        .route(
            "/api/hosting/apps/{id}/{generation}/results/collections/{collection}/{*name}",
            get(collection_file),
        )
        .with_state(state)
        .fallback_service(ServeDir::new(assets).append_index_html_on_directories(true))
}
fn authorized(headers: &HeaderMap, origin: &str) -> bool {
    headers.get_all(header::ORIGIN).iter().count() == 1
        && headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) == Some(origin)
}
fn authorized_upstream_method(headers: &HeaderMap, origin: &str) -> Result<(), StatusCode> {
    let methods = headers.get_all("x-forwarded-method");
    if methods.iter().count() != 1 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let method = methods
        .iter()
        .next()
        .and_then(|value| axum::http::Method::from_bytes(value.as_bytes()).ok())
        .ok_or(StatusCode::BAD_REQUEST)?;
    if matches!(
        method,
        axum::http::Method::GET | axum::http::Method::HEAD | axum::http::Method::OPTIONS
    ) || authorized(headers, origin)
    {
        Ok(())
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}
fn token(value: &str) -> Option<Token> {
    if value.len() != 32
        || !value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return None;
    }
    Token::from_str_radix(value, 16).ok()
}
fn error(error: Error) -> (StatusCode, Json<Value>) {
    match error {
        Error::UnknownApp => (StatusCode::NOT_FOUND, Json(json!({"error":"unknown app"}))),
        Error::ShuttingDown => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error":"server shutting down"})),
        ),
        Error::Blocked(message) => (StatusCode::CONFLICT, Json(json!({"error":message}))),
        Error::Conflict { owner, resource } => (
            StatusCode::CONFLICT,
            Json(json!({"error":format!("resource {resource} is occupied by {owner}")})),
        ),
    }
}
fn status_value(status: Status) -> Value {
    match status {
        Status::Stopped { error } => json!({"state":"stopped","error":error}),
        Status::Starting { generation } => {
            json!({"state":"starting","generation":format!("{generation:032x}")})
        }
        Status::Ready { generation } => {
            json!({"state":"ready","generation":format!("{generation:032x}")})
        }
        Status::Stopping { generation } => {
            json!({"state":"stopping","generation":format!("{generation:032x}")})
        }
        Status::Blocked { error } => json!({"state":"blocked","error":error}),
    }
}
async fn viewer() -> Html<&'static str> {
    Html(include_str!("viewer.html"))
}

async fn authorize_upstream(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(status) = authorized_upstream_method(&headers, &state.origin) {
        return (status, HeaderMap::new());
    }
    let values = headers.get_all("x-forwarded-uri");
    if values.iter().count() != 1 {
        return (StatusCode::BAD_REQUEST, HeaderMap::new());
    }
    let Some(uri) = values
        .iter()
        .next()
        .and_then(|h| h.to_str().ok())
        .and_then(|u| u.parse::<axum::http::Uri>().ok())
    else {
        return (StatusCode::BAD_REQUEST, HeaderMap::new());
    };
    let mut segments = uri.path().split('/');
    let (Some(""), Some("app"), Some(id), Some(generation)) = (
        segments.next(),
        segments.next(),
        segments.next(),
        segments.next(),
    ) else {
        return (StatusCode::NOT_FOUND, HeaderMap::new());
    };
    if segments.next().is_none() {
        return (StatusCode::NOT_FOUND, HeaderMap::new());
    }
    let Some(generation) = token(generation) else {
        return (StatusCode::NOT_FOUND, HeaderMap::new());
    };
    match state.service.upstream(id, generation).await {
        Ok(Some(upstream)) => {
            let mut response = HeaderMap::new();
            if let Ok(value) = upstream.to_string().parse() {
                response.insert("x-gallery-upstream", value);
                return (StatusCode::NO_CONTENT, response);
            }
            (StatusCode::BAD_GATEWAY, HeaderMap::new())
        }
        _ => (StatusCode::NOT_FOUND, HeaderMap::new()),
    }
}

async fn apps(State(state): State<ApiState>) -> Json<Value> {
    Json(json!({"apps": state.service.app_ids(), "heartbeat_seconds": state.heartbeat_seconds}))
}
async fn status(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let headless = state.service.is_headless(&id).map_err(error)?;
    let result_format = state.service.result_format(&id).map_err(error)?;
    state
        .service
        .status(&id)
        .await
        .map(|s| {
            let mut value = status_value(s);
            value["headless"] = json!(headless);
            value["result_format"] = json!(match result_format {
                crate::registry::ResultFormat::Mcap => "mcap",
                crate::registry::ResultFormat::File => "file",
            });
            Json(value)
        })
        .map_err(error)
}
async fn launch(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(_request): Json<EmptyRequest>,
) -> impl IntoResponse {
    if !authorized(&headers, &state.origin) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error":"origin rejected"})),
        );
    }
    match state.service.launch(&id).await {
        Ok(launch) => (
            StatusCode::ACCEPTED,
            Json(
                json!({"generation":format!("{:032x}",launch.generation),"session":format!("{:032x}",launch.session)}),
            ),
        ),
        Err(e) => error(e),
    }
}
fn open_regular(path: &std::path::Path) -> Option<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .ok()?;
    if file.metadata().ok()?.is_file() {
        Some(file)
    } else {
        None
    }
}
fn marker_verified(path: &std::path::Path) -> bool {
    use std::io::Read;
    let Some(file) = open_regular(path) else {
        return false;
    };
    let mut contents = Vec::new();
    file.take(10).read_to_end(&mut contents).is_ok() && contents == b"verified\n"
}
fn file_response(file: std::fs::File, filename: &str) -> axum::response::Response {
    let size = match file.metadata() {
        Ok(metadata) => metadata.len(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let disposition = format!("attachment; filename=\"{filename}\"");
    let stream = tokio_util::io::ReaderStream::new(tokio::fs::File::from_std(file));
    match axum::response::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_LENGTH, size)
        .header(header::CONTENT_DISPOSITION, disposition)
        .body(Body::from_stream(stream))
    {
        Ok(response) => response,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
fn result_directory(state: &ApiState, id: &str, generation: Token) -> Option<std::path::PathBuf> {
    let directory = state.service.captures_directory(id, generation).ok()??;
    if std::fs::symlink_metadata(&directory)
        .ok()?
        .file_type()
        .is_dir()
    {
        Some(directory)
    } else {
        None
    }
}
async fn recording(
    State(state): State<ApiState>,
    Path((id, generation)): Path<(String, String)>,
) -> axum::response::Response {
    let Some(generation) = token(&generation) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok((marker, path, filename)) = state.service.recording_paths(&id, generation) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !marker_verified(&marker)
        || !state
            .service
            .primary_present(&id, generation)
            .unwrap_or(false)
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    match open_regular(&path) {
        Some(file) => file_response(file, &filename),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
async fn capture(
    State(state): State<ApiState>,
    Path((id, generation, name)): Path<(String, String, String)>,
) -> axum::response::Response {
    let Some(generation) = token(&generation) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !crate::workflow::capture_name(&name) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Ok((marker, _, _)) = state.service.recording_paths(&id, generation) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !marker_verified(&marker) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(directory) = result_directory(&state, &id, generation) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match open_regular(&directory.join(&name)) {
        Some(file) => file_response(file, &name),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
async fn result_list(
    State(state): State<ApiState>,
    Path((id, generation)): Path<(String, String)>,
) -> axum::response::Response {
    let Some(generation) = token(&generation) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok((marker, primary, _)) = state.service.recording_paths(&id, generation) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !marker_verified(&marker) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let mut captures = Vec::new();
    if let Some(directory) = result_directory(&state, &id, generation) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        };
        for entry in entries {
            let Ok(entry) = entry else {
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            };
            if let Some(name) = entry
                .file_name()
                .to_str()
                .filter(|name| crate::workflow::capture_name(name))
                && open_regular(&entry.path()).is_some()
            {
                captures.push(name.to_owned());
            }
        }
        captures.sort();
    }
    let primary_exists = state
        .service
        .primary_present(&id, generation)
        .unwrap_or(false)
        && open_regular(&primary).is_some();
    let recording = primary_exists
        && matches!(
            state.service.result_format(&id),
            Ok(crate::registry::ResultFormat::Mcap)
        );
    let files: Vec<_> = state
        .service
        .additional_files(&id, generation)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(name, path)| open_regular(&path).map(|_| name))
        .collect();
    let mut result =
        json!({"recording":recording,"file":primary_exists && !recording,"captures":captures});
    if !files.is_empty() {
        result["files"] = json!(files);
    }
    let root = primary
        .parent()
        .and_then(|dir| crate::artifact_fs::root(dir).ok());
    let mut collections = state
        .service
        .artifact_collections(&id, generation)
        .unwrap_or_default();
    for collection in &mut collections {
        collection.files.retain(|name| {
            root.as_ref()
                .is_some_and(|dir| crate::artifact_fs::open(dir, name, false).is_ok())
        });
    }
    collections.retain(|collection| !collection.files.is_empty());
    if !collections.is_empty() {
        result["collections"] = json!(collections);
    }
    Json(result).into_response()
}

async fn collection_file(
    State(state): State<ApiState>,
    Path((id, generation, collection, name)): Path<(String, String, String, String)>,
) -> axum::response::Response {
    let Some(generation) = token(&generation) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok((marker, primary, _)) = state.service.recording_paths(&id, generation) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !marker_verified(&marker) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Ok(collections) = state.service.artifact_collections(&id, generation) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !collections
        .iter()
        .any(|entry| entry.id == collection && entry.files.contains(&name))
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(directory) = primary.parent() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !crate::registry::artifact_path(&name) {
        return StatusCode::NOT_FOUND.into_response();
    }
    match crate::artifact_fs::root(directory)
        .and_then(|root| crate::artifact_fs::open(&root, &name, false))
    {
        Ok(file) => file_response(file, name.rsplit('/').next().unwrap_or(&name)),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn additional_file(
    State(state): State<ApiState>,
    Path((id, generation, name)): Path<(String, String, String)>,
) -> axum::response::Response {
    let Some(generation) = token(&generation) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok((marker, _, _)) = state.service.recording_paths(&id, generation) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !marker_verified(&marker) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Ok(files) = state.service.additional_files(&id, generation) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some((_, path)) = files.into_iter().find(|(declared, _)| declared == &name) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match open_regular(&path) {
        Some(file) => file_response(file, &name),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn heartbeat(
    State(state): State<ApiState>,
    Path((id, generation)): Path<(String, String)>,
    headers: HeaderMap,
    Json(request): Json<HeartbeatRequest>,
) -> impl IntoResponse {
    if !authorized(&headers, &state.origin) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error":"origin rejected"})),
        );
    }
    let (Some(generation), Some(session)) = (token(&generation), token(&request.session)) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"invalid token"})),
        );
    };
    match state.service.heartbeat(&id, generation, session).await {
        Ok(true) => (StatusCode::OK, Json(json!({"active":true}))),
        Ok(false) => (StatusCode::GONE, Json(json!({"active":false}))),
        Err(e) => error(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn origin_tokens_and_request_shapes_are_strict() {
        // Arrange
        let mut headers = HeaderMap::new();
        headers.insert(header::ORIGIN, "https://gallery.example".parse().unwrap());

        // Act / Assert
        assert!(authorized(&headers, "https://gallery.example"));
        assert!(!authorized(&headers, "https://other.example"));
        assert!(token("00000000000000000000000000000001").is_some());
        assert!(token("0000000000000000000000000000000A").is_none());
        assert!(token("1").is_none());
        assert!(serde_json::from_str::<EmptyRequest>("{}").is_ok());
        assert!(serde_json::from_str::<EmptyRequest>(r#"{"compose":"untrusted"}"#).is_err());
        headers.append(header::ORIGIN, "https://gallery.example".parse().unwrap());
        assert!(!authorized(&headers, "https://gallery.example"));
    }
    #[test]
    fn upstream_mutations_require_exact_origin() {
        // Arrange
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-method", "GET".parse().unwrap());
        // Act / Assert
        assert!(authorized_upstream_method(&headers, "https://gallery.example").is_ok());
        headers.insert("x-forwarded-method", "POST".parse().unwrap());
        assert_eq!(
            authorized_upstream_method(&headers, "https://gallery.example"),
            Err(StatusCode::FORBIDDEN)
        );
        headers.insert(header::ORIGIN, "https://gallery.example".parse().unwrap());
        assert!(authorized_upstream_method(&headers, "https://gallery.example").is_ok());
        headers.append(header::ORIGIN, "https://gallery.example".parse().unwrap());
        assert_eq!(
            authorized_upstream_method(&headers, "https://gallery.example"),
            Err(StatusCode::FORBIDDEN)
        );
        headers.remove("x-forwarded-method");
        assert_eq!(
            authorized_upstream_method(&headers, "https://gallery.example"),
            Err(StatusCode::BAD_REQUEST)
        );
    }
}
