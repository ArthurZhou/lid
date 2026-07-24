use axum::{
    extract::Extension,
    http::{header, HeaderMap},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::auth::jwt;
use crate::auth::password;
use crate::auth::AuthCtx;
use crate::auth::oidc::OidcProvider;
use crate::config::Config;
use crate::db::DbPool;
use crate::db::models::User;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub email: String,
    pub is_admin: bool,
}

/// POST /api/auth/login
pub async fn login(
    Extension(pool): Extension<DbPool>,
    Extension(config): Extension<Arc<Config>>,
    Json(payload): Json<LoginRequest>,
) -> Result<(HeaderMap, Json<Value>), AppError> {
    let conn = pool.get()?;

    let user = User::find_by_username(&conn, &payload.username)?
        .ok_or_else(|| AppError::Unauthorized("Invalid username or password".to_string()))?;

    let valid = password::verify_password(&payload.password, &user.password_hash)?;
    if !valid {
        return Err(AppError::Unauthorized(
            "Invalid username or password".to_string(),
        ));
    }

    let token = jwt::create_token(&user.id, &config.auth.jwt_secret, config.auth.session_days)?;

    let cookie = format!(
        "lid_token={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        token,
        config.auth.session_days * 24 * 60 * 60
    );

    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, cookie.parse().unwrap());

    let resp = json!({
        "data": {
            "user": {
                "id": user.id,
                "username": user.username,
                "email": user.email,
                "is_admin": user.is_admin,
            },
            "token": token,
        }
    });

    Ok((headers, Json(resp)))
}

/// POST /api/auth/logout
pub async fn logout() -> Result<(HeaderMap, Json<Value>), AppError> {
    let cookie = "lid_token=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0";

    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, cookie.parse().unwrap());

    Ok((headers, Json(json!({ "data": "logged out" }))))
}

/// GET /api/auth/me
pub async fn me(
    Extension(auth): Extension<AuthCtx>,
    Extension(pool): Extension<DbPool>,
) -> Result<Json<Value>, AppError> {
    let conn = pool.get()?;

    let user = User::find_by_id(&conn, &auth.user_id)?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    Ok(Json(json!({
        "data": {
            "id": user.id,
            "username": user.username,
            "email": user.email,
            "is_admin": user.is_admin,
        }
    })))
}

/// GET /api/auth/oidc/login
pub async fn oidc_login(
    Extension(config): Extension<Arc<Config>>,
    Extension(oidc_provider): Extension<Option<OidcProvider>>,
) -> Result<Json<Value>, AppError> {
    if !config.auth.oidc.enabled {
        return Err(AppError::BadRequest("OIDC is not enabled".to_string()));
    }

    let provider = oidc_provider
        .ok_or_else(|| AppError::InternalError("OIDC provider not initialized".to_string()))?;

    let redirect_uri = format!(
        "{}api/auth/oidc/callback",
        config.server.base_url.trim_end_matches('/')
    );

    // For a real deployment this would be an HTTP redirect.
    // Return the URL so the frontend can redirect.
    let auth_url = provider.auth_url(&redirect_uri);

    Ok(Json(json!({
        "data": {
            "redirect_url": auth_url,
        }
    })))
}

#[derive(Debug, Deserialize)]
pub struct OidcCallbackParams {
    pub code: String,
    #[allow(dead_code)]
    pub state: Option<String>,
}

/// GET /api/auth/oidc/callback
pub async fn oidc_callback(
    axum::extract::Query(params): axum::extract::Query<OidcCallbackParams>,
    Extension(config): Extension<Arc<Config>>,
    Extension(pool): Extension<DbPool>,
    Extension(oidc_provider): Extension<Option<OidcProvider>>,
) -> Result<(HeaderMap, Json<Value>), AppError> {
    if !config.auth.oidc.enabled {
        return Err(AppError::BadRequest("OIDC is not enabled".to_string()));
    }

    let provider = oidc_provider
        .ok_or_else(|| AppError::InternalError("OIDC provider not initialized".to_string()))?;

    let redirect_uri = format!(
        "{}api/auth/oidc/callback",
        config.server.base_url.trim_end_matches('/')
    );

    // Exchange code for token
    let access_token = provider.exchange_code(&params.code, &redirect_uri).await?;

    // Fetch user info
    let oidc_user = provider.fetch_userinfo(&access_token).await?;

    let conn = pool.get()?;

    // Find or create user
    let user = match User::find_by_oidc_subject(&conn, &oidc_user.subject)? {
        Some(user) => user,
        None => {
            // Auto-provision
            let user_id = uuid::Uuid::new_v4().to_string();
            let username = oidc_user
                .name
                .unwrap_or_else(|| oidc_user.subject.clone());
            let email = oidc_user.email.unwrap_or_default();
            let now = chrono::Utc::now().timestamp();

            User::create(
                &conn,
                &user_id,
                &username,
                "",
                &email,
                Some(&oidc_user.subject),
                false,
                now,
            )?;

            User::find_by_id(&conn, &user_id)?
                .ok_or_else(|| AppError::InternalError("Failed to create OIDC user".to_string()))?
        }
    };

    let token = jwt::create_token(&user.id, &config.auth.jwt_secret, config.auth.session_days)?;

    let cookie = format!(
        "lid_token={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        token,
        config.auth.session_days * 24 * 60 * 60
    );

    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, cookie.parse().unwrap());

    let resp = json!({
        "data": {
            "user": {
                "id": user.id,
                "username": user.username,
                "email": user.email,
                "is_admin": user.is_admin,
            },
            "token": token,
        }
    });

    Ok((headers, Json(resp)))
}
