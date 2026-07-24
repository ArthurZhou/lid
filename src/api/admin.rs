use axum::{
    extract::{Extension, Path},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::password;
use crate::auth::AuthCtx;
use crate::db::DbPool;
use crate::db::models::{Group, PathEntry, Permission, User};
use crate::error::AppError;

/// Macro to check admin access.
fn require_admin(auth: &AuthCtx) -> Result<(), AppError> {
    if !auth.is_admin {
        Err(AppError::Forbidden("Admin access required".to_string()))
    } else {
        Ok(())
    }
}

// ── Users ─────────────────────────────────────────────

/// GET /api/admin/users
pub async fn list_users(
    Extension(auth): Extension<AuthCtx>,
    Extension(pool): Extension<DbPool>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let conn = pool.get()?;
    let users = User::list_all(&conn)?;
    Ok(Json(json!({ "data": users })))
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub is_admin: bool,
}

/// POST /api/admin/users
pub async fn create_user(
    Extension(auth): Extension<AuthCtx>,
    Extension(pool): Extension<DbPool>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;

    let conn = pool.get()?;

    // Check if username already exists
    if User::find_by_username(&conn, &payload.username)?.is_some() {
        return Err(AppError::BadRequest("Username already exists".to_string()));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let hash = password::hash_password(&payload.password)?;
    let now = chrono::Utc::now().timestamp();

    User::create(
        &conn,
        &id,
        &payload.username,
        &hash,
        &payload.email,
        None,
        payload.is_admin,
        now,
    )?;

    let user = User::find_by_id(&conn, &id)?;

    Ok(Json(json!({ "data": user })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub username: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub is_admin: bool,
    pub password: Option<String>,
}

/// PUT /api/admin/users/:id
pub async fn update_user(
    Path(user_id): Path<String>,
    Extension(auth): Extension<AuthCtx>,
    Extension(pool): Extension<DbPool>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;

    let conn = pool.get()?;

    User::update(&conn, &user_id, &payload.username, &payload.email, payload.is_admin)?;

    if let Some(ref pw) = payload.password {
        if !pw.is_empty() {
            let hash = password::hash_password(pw)?;
            User::update_password(&conn, &user_id, &hash)?;
        }
    }

    let user = User::find_by_id(&conn, &user_id)?;

    Ok(Json(json!({ "data": user })))
}

/// DELETE /api/admin/users/:id
pub async fn delete_user(
    Path(user_id): Path<String>,
    Extension(auth): Extension<AuthCtx>,
    Extension(pool): Extension<DbPool>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;

    if auth.user_id == user_id {
        return Err(AppError::BadRequest("Cannot delete yourself".to_string()));
    }

    let conn = pool.get()?;
    User::delete(&conn, &user_id)?;

    Ok(Json(json!({ "data": "user deleted" })))
}

// ── Groups ────────────────────────────────────────────

/// GET /api/admin/groups
pub async fn list_groups(
    Extension(auth): Extension<AuthCtx>,
    Extension(pool): Extension<DbPool>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let conn = pool.get()?;
    let groups = Group::list_all(&conn)?;
    Ok(Json(json!({ "data": groups })))
}

#[derive(Debug, Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
}

/// POST /api/admin/groups
pub async fn create_group(
    Extension(auth): Extension<AuthCtx>,
    Extension(pool): Extension<DbPool>,
    Json(payload): Json<CreateGroupRequest>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;

    let conn = pool.get()?;
    let id = uuid::Uuid::new_v4().to_string();
    Group::create(&conn, &id, &payload.name)?;

    let group = Group::find_by_id(&conn, &id)?;

    Ok(Json(json!({ "data": group })))
}

/// DELETE /api/admin/groups/:id
pub async fn delete_group(
    Path(group_id): Path<String>,
    Extension(auth): Extension<AuthCtx>,
    Extension(pool): Extension<DbPool>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;

    let conn = pool.get()?;
    Group::delete(&conn, &group_id)?;

    Ok(Json(json!({ "data": "group deleted" })))
}

// ── Paths ─────────────────────────────────────────────

/// GET /api/admin/paths
pub async fn list_paths(
    Extension(auth): Extension<AuthCtx>,
    Extension(pool): Extension<DbPool>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let conn = pool.get()?;
    let paths = PathEntry::list_all(&conn)?;
    Ok(Json(json!({ "data": paths })))
}

#[derive(Debug, Deserialize)]
pub struct AddPathRequest {
    pub path: String,
    pub display_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub is_public: bool,
}

/// POST /api/admin/paths
pub async fn add_path(
    Extension(auth): Extension<AuthCtx>,
    Extension(pool): Extension<DbPool>,
    Json(payload): Json<AddPathRequest>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;

    // Verify path exists on filesystem
    let p = std::path::Path::new(&payload.path);
    if !p.exists() {
        return Err(AppError::BadRequest(format!(
            "Path does not exist: {}",
            payload.path
        )));
    }

    let conn = pool.get()?;
    let id = uuid::Uuid::new_v4().to_string();

    PathEntry::create(
        &conn,
        &id,
        &payload.path,
        &payload.display_name,
        &payload.description,
        payload.is_public,
    )?;

    let entry = PathEntry::find_by_id(&conn, &id)?;

    Ok(Json(json!({ "data": entry })))
}

/// DELETE /api/admin/paths/:id
pub async fn remove_path(
    Path(path_id): Path<String>,
    Extension(auth): Extension<AuthCtx>,
    Extension(pool): Extension<DbPool>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;

    let conn = pool.get()?;
    PathEntry::delete(&conn, &path_id)?;

    Ok(Json(json!({ "data": "path removed" })))
}

// ── Permissions ───────────────────────────────────────

/// GET /api/admin/permissions
pub async fn list_permissions(
    Extension(auth): Extension<AuthCtx>,
    Extension(pool): Extension<DbPool>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let conn = pool.get()?;
    let perms = Permission::list_all(&conn)?;
    Ok(Json(json!({ "data": perms })))
}

#[derive(Debug, Deserialize)]
pub struct GrantPermissionRequest {
    pub principal_type: String,
    pub principal_id: String,
    pub path_id: String,
    pub permission: String,
}

/// POST /api/admin/permissions
pub async fn grant_permission(
    Extension(auth): Extension<AuthCtx>,
    Extension(pool): Extension<DbPool>,
    Json(payload): Json<GrantPermissionRequest>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;

    // Validate principal_type
    if payload.principal_type != "user" && payload.principal_type != "group" {
        return Err(AppError::BadRequest(
            "principal_type must be 'user' or 'group'".to_string(),
        ));
    }

    // Validate permission level
    if !["read", "write", "admin"].contains(&payload.permission.as_str()) {
        return Err(AppError::BadRequest(
            "permission must be 'read', 'write', or 'admin'".to_string(),
        ));
    }

    let conn = pool.get()?;
    let id = uuid::Uuid::new_v4().to_string();

    Permission::create(
        &conn,
        &id,
        &payload.principal_type,
        &payload.principal_id,
        &payload.path_id,
        &payload.permission,
    )?;

    let perm = Permission::find_by_id(&conn, &id)?;

    Ok(Json(json!({ "data": perm })))
}

/// DELETE /api/admin/permissions/:id
pub async fn revoke_permission(
    Path(perm_id): Path<String>,
    Extension(auth): Extension<AuthCtx>,
    Extension(pool): Extension<DbPool>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;

    let conn = pool.get()?;
    Permission::delete(&conn, &perm_id)?;

    Ok(Json(json!({ "data": "permission revoked" })))
}
