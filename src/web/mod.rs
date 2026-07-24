use axum::{
    Router,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get_service,
};
use std::path::PathBuf;
use tower_http::services::ServeDir;

/// Builds a router that serves static files at /*path
/// (no /static prefix - caller should nest this at desired path)
pub fn build_web_router(static_dir: PathBuf) -> Router {
    Router::new().route(
        "/*path",
        get_service(ServeDir::new(static_dir)),
    )
}

/// SPA fallback handler - serves index.html for any non-API path
pub async fn spa_fallback() -> Response {
    let index_path = PathBuf::from("static/index.html");
    match tokio::fs::read_to_string(&index_path).await {
        Ok(contents) => axum::response::Html(contents).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}