use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;

use crate::auth::jwt;
use crate::auth::AuthCtx;
use crate::config::Config;
use crate::db::DbPool;

/// Auth middleware: validates JWT and injects AuthCtx.
/// Skips public routes.
/// Reads Arc<Config> from request extensions (injected by build_router).
pub async fn auth_middleware(
    req: Request,
    next: Next,
) -> Result<Response, Response> {
    let path = req.uri().path().to_string();

    // Skip auth for public routes
    if path.starts_with("/api/auth/login")
        || path.starts_with("/api/auth/oidc")
        || path.starts_with("/static")
        || path.starts_with("/api/files/public")
        || !path.starts_with("/api/")
        || path == "/health"
    {
        return Ok(next.run(req).await);
    }

    // Get config from extensions
    let config = req
        .extensions()
        .get::<Arc<Config>>()
        .cloned()
        .ok_or_else(|| {
            tracing::error!("[auth] Arc<Config> extension missing");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Server configuration error"})),
            )
                .into_response()
        })?;

    // Get pool from extensions
    let pool = req
        .extensions()
        .get::<DbPool>()
        .cloned()
        .ok_or_else(|| {
            tracing::error!("[auth] DbPool extension missing");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database connection error"})),
            )
                .into_response()
        })?;

    // Extract token from Cookie or Authorization header
    let token = extract_token(&req);

    let token = match token {
        Some(t) => t,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Authentication required"})),
            )
                .into_response());
        }
    };

    // Verify JWT
    let claims = jwt::verify_token(&token, &config.auth.jwt_secret).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid or expired token"})),
        )
            .into_response()
    })?;

    // Look up user
    let conn = pool.get().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database connection error"})),
        )
            .into_response()
    })?;

    let user = crate::db::models::User::find_by_id(&conn, &claims.sub).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
            .into_response()
    })?;

    let user = match user {
        Some(u) => u,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "User not found"})),
            )
                .into_response());
        }
    };

    // Inject AuthCtx into request extensions for downstream handlers
    let auth_ctx = AuthCtx {
        user_id: user.id.clone(),
        username: user.username.clone(),
        is_admin: user.is_admin,
    };

    let mut req = req;
    req.extensions_mut().insert(auth_ctx);

    Ok(next.run(req).await)
}

fn extract_token(req: &Request) -> Option<String> {
    // Try Authorization header first
    if let Some(auth_header) = req.headers().get("authorization") {
        if let Ok(value) = auth_header.to_str() {
            if let Some(token) = value.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }

    // Try Cookie header
    if let Some(cookie_header) = req.headers().get("cookie") {
        if let Ok(value) = cookie_header.to_str() {
            for cookie_str in value.split(';') {
                let cookie_str = cookie_str.trim();
                if let Some(token) = cookie_str.strip_prefix("lid_token=") {
                    return Some(token.to_string());
                }
            }
        }
    }

    None
}