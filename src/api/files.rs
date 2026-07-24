use axum::{
    body::Body,
    extract::{Extension, Multipart, Path},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio_util::io::ReaderStream;

use crate::acl::checker::AclChecker;
use crate::auth::AuthCtx;
use crate::db::DbPool;
use crate::db::models::PathEntry;
use crate::error::AppError;

#[derive(Debug, serde::Serialize)]
struct FileInfo {
    name: String,
    size: u64,
    modified: i64,
    is_dir: bool,
}

/// GET /api/files - list user's accessible paths
pub async fn list_paths(
    Extension(auth): Extension<AuthCtx>,
    Extension(acl): Extension<AclChecker>,
    Extension(pool): Extension<DbPool>,
) -> Result<Json<Value>, AppError> {
    let paths = if auth.is_admin {
        let conn = pool.get()?;
        PathEntry::list_all(&conn)?
    } else {
        acl.list_accessible_paths(&auth.user_id)?
    };

    Ok(Json(json!({ "data": paths })))
}

/// GET /api/files/:path_id - list directory contents
pub async fn list_dir(
    Path((path_id, sub_path)): Path<(String, String)>,
    Extension(auth): Extension<AuthCtx>,
    Extension(acl): Extension<AclChecker>,
    Extension(pool): Extension<DbPool>,
) -> Result<Json<Value>, AppError> {
    // Check read permission
    if !auth.is_admin && !acl.check(&auth.user_id, &path_id, "read")? {
        return Err(AppError::Forbidden("Access denied".to_string()));
    }

    let conn = pool.get()?;
    let path_entry = PathEntry::find_by_id(&conn, &path_id)?
        .ok_or_else(|| AppError::NotFound("Path not found".to_string()))?;

    let mut dir_path = PathBuf::from(&path_entry.path);
    if !sub_path.is_empty() && sub_path != "/" {
        // Sanitize sub_path to prevent directory traversal
        let clean_sub = sub_path.trim_start_matches('/');
        if clean_sub.contains("..") {
            return Err(AppError::BadRequest("Invalid path".to_string()));
        }
        dir_path.push(clean_sub);
    }

    if !dir_path.is_dir() {
        return Err(AppError::NotFound("Directory not found".to_string()));
    }

    let mut entries = Vec::new();
    let mut read_dir = tokio::fs::read_dir(&dir_path).await?;

    while let Some(entry) = read_dir.next_entry().await? {
        let metadata = entry.metadata().await?;
        let modified = metadata
            .modified()
            .map(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64
            })
            .unwrap_or(0);

        entries.push(FileInfo {
            name: entry.file_name().to_string_lossy().to_string(),
            size: metadata.len(),
            modified,
            is_dir: metadata.is_dir(),
        });
    }

    // Sort: dirs first, then by name
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(Json(json!({
        "data": {
            "path": path_entry.display_name,
            "sub_path": sub_path,
            "entries": entries,
        }
    })))
}

/// GET /api/files/:path_id/browse/* - list directory (root level)
pub async fn list_dir_root(
    Path(path_id): Path<String>,
    Extension(auth): Extension<AuthCtx>,
    Extension(acl): Extension<AclChecker>,
    Extension(pool): Extension<DbPool>,
) -> Result<Json<Value>, AppError> {
    list_dir(
        Path((path_id, String::new())),
        Extension(auth),
        Extension(acl),
        Extension(pool),
    )
    .await
}

/// GET /api/files/:path_id/download/*filename - download file
pub async fn download(
    Path((path_id, filename)): Path<(String, String)>,
    Extension(auth): Extension<AuthCtx>,
    Extension(acl): Extension<AclChecker>,
    Extension(pool): Extension<DbPool>,
) -> Result<Response, AppError> {
    if !auth.is_admin && !acl.check(&auth.user_id, &path_id, "read")? {
        return Err(AppError::Forbidden("Access denied".to_string()));
    }

    let conn = pool.get()?;
    let path_entry = PathEntry::find_by_id(&conn, &path_id)?
        .ok_or_else(|| AppError::NotFound("Path not found".to_string()))?;

    // Sanitize filename
    if filename.contains("..") {
        return Err(AppError::BadRequest("Invalid filename".to_string()));
    }

    let file_path = PathBuf::from(&path_entry.path).join(&filename);

    if !file_path.is_file() {
        return Err(AppError::NotFound("File not found".to_string()));
    }

    let file = tokio::fs::File::open(&file_path).await?;
    let metadata = file.metadata().await?;
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    // Guess MIME type
    let mime_type = mime_guess::from_path(&file_path)
        .first_or_octet_stream()
        .to_string();

    let basename = file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, &mime_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", basename),
        )
        .header(header::CONTENT_LENGTH, metadata.len())
        .body(body)
        .map_err(|e| AppError::InternalError(e.to_string()))?;

    Ok(response)
}

/// HEAD /api/files/:path_id/download/*filename
pub async fn head_file(
    Path((path_id, filename)): Path<(String, String)>,
    Extension(auth): Extension<AuthCtx>,
    Extension(acl): Extension<AclChecker>,
    Extension(pool): Extension<DbPool>,
) -> Result<Response, AppError> {
    if !auth.is_admin && !acl.check(&auth.user_id, &path_id, "read")? {
        return Err(AppError::Forbidden("Access denied".to_string()));
    }

    let conn = pool.get()?;
    let path_entry = PathEntry::find_by_id(&conn, &path_id)?
        .ok_or_else(|| AppError::NotFound("Path not found".to_string()))?;

    if filename.contains("..") {
        return Err(AppError::BadRequest("Invalid filename".to_string()));
    }

    let file_path = PathBuf::from(&path_entry.path).join(&filename);

    if !file_path.is_file() {
        return Err(AppError::NotFound("File not found".to_string()));
    }

    let metadata = tokio::fs::metadata(&file_path).await?;
    let mime_type = mime_guess::from_path(&file_path)
        .first_or_octet_stream()
        .to_string();

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, &mime_type)
        .header(header::CONTENT_LENGTH, metadata.len())
        .body(Body::empty())
        .map_err(|e| AppError::InternalError(e.to_string()))?;

    Ok(response)
}

#[derive(Debug, Deserialize)]
pub struct MkdirRequest {
    pub name: String,
}

/// POST /api/files/:path_id/mkdir
pub async fn mkdir(
    Path(path_id): Path<String>,
    Extension(auth): Extension<AuthCtx>,
    Extension(acl): Extension<AclChecker>,
    Extension(pool): Extension<DbPool>,
    Json(payload): Json<MkdirRequest>,
) -> Result<Json<Value>, AppError> {
    if !auth.is_admin && !acl.check(&auth.user_id, &path_id, "write")? {
        return Err(AppError::Forbidden("Access denied".to_string()));
    }

    let conn = pool.get()?;
    let path_entry = PathEntry::find_by_id(&conn, &path_id)?
        .ok_or_else(|| AppError::NotFound("Path not found".to_string()))?;

    if payload.name.contains("..") || payload.name.contains('/') {
        return Err(AppError::BadRequest("Invalid directory name".to_string()));
    }

    let dir_path = PathBuf::from(&path_entry.path).join(&payload.name);
    tokio::fs::create_dir_all(&dir_path).await?;

    Ok(Json(json!({ "data": "directory created" })))
}

#[derive(Debug, Deserialize)]
pub struct RenameRequest {
    pub old_name: String,
    pub new_name: String,
}

/// POST /api/files/:path_id/rename
pub async fn rename(
    Path(path_id): Path<String>,
    Extension(auth): Extension<AuthCtx>,
    Extension(acl): Extension<AclChecker>,
    Extension(pool): Extension<DbPool>,
    Json(payload): Json<RenameRequest>,
) -> Result<Json<Value>, AppError> {
    if !auth.is_admin && !acl.check(&auth.user_id, &path_id, "write")? {
        return Err(AppError::Forbidden("Access denied".to_string()));
    }

    let conn = pool.get()?;
    let path_entry = PathEntry::find_by_id(&conn, &path_id)?
        .ok_or_else(|| AppError::NotFound("Path not found".to_string()))?;

    for name in [&payload.old_name, &payload.new_name] {
        if name.contains("..") || name.contains('/') {
            return Err(AppError::BadRequest("Invalid name".to_string()));
        }
    }

    let old_path = PathBuf::from(&path_entry.path).join(&payload.old_name);
    let new_path = PathBuf::from(&path_entry.path).join(&payload.new_name);

    if !old_path.exists() {
        return Err(AppError::NotFound("Source not found".to_string()));
    }

    tokio::fs::rename(&old_path, &new_path).await?;

    Ok(Json(json!({ "data": "renamed" })))
}

/// DELETE /api/files/:path_id/delete/:name
pub async fn delete(
    Path((path_id, name)): Path<(String, String)>,
    Extension(auth): Extension<AuthCtx>,
    Extension(acl): Extension<AclChecker>,
    Extension(pool): Extension<DbPool>,
) -> Result<Json<Value>, AppError> {
    if !auth.is_admin && !acl.check(&auth.user_id, &path_id, "write")? {
        return Err(AppError::Forbidden("Access denied".to_string()));
    }

    let conn = pool.get()?;
    let path_entry = PathEntry::find_by_id(&conn, &path_id)?
        .ok_or_else(|| AppError::NotFound("Path not found".to_string()))?;

    if name.contains("..") || name.contains('/') {
        return Err(AppError::BadRequest("Invalid name".to_string()));
    }

    let target = PathBuf::from(&path_entry.path).join(&name);

    if !target.exists() {
        return Err(AppError::NotFound("File/directory not found".to_string()));
    }

    if target.is_dir() {
        tokio::fs::remove_dir_all(&target).await?;
    } else {
        tokio::fs::remove_file(&target).await?;
    }

    Ok(Json(json!({ "data": "deleted" })))
}

/// POST /api/files/:path_id/upload (multipart form)
pub async fn upload(
    Path(path_id): Path<String>,
    Extension(auth): Extension<AuthCtx>,
    Extension(acl): Extension<AclChecker>,
    Extension(pool): Extension<DbPool>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    if !auth.is_admin && !acl.check(&auth.user_id, &path_id, "write")? {
        return Err(AppError::Forbidden("Access denied".to_string()));
    }

    let conn = pool.get()?;
    let path_entry = PathEntry::find_by_id(&conn, &path_id)?
        .ok_or_else(|| AppError::NotFound("Path not found".to_string()))?;

    let mut uploaded = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart error: {}", e)))?
    {
        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unnamed".to_string());

        if filename.contains("..") || filename.contains('/') {
            return Err(AppError::BadRequest("Invalid filename".to_string()));
        }

        let file_path = PathBuf::from(&path_entry.path).join(&filename);

        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("Failed to read field: {}", e)))?;

        tokio::fs::write(&file_path, &data).await?;
        uploaded.push(filename);
    }

    Ok(Json(json!({ "data": uploaded })))
}

