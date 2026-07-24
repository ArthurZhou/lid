use axum::{
    Router,
    routing::{get, post, delete, put},
    Extension,
};
use std::sync::Arc;
use tower_http::cors::{CorsLayer, AllowOrigin};
use axum::http::{Method, HeaderName};

use crate::acl::checker::AclChecker;
use crate::auth::middleware::auth_middleware;
use crate::auth::oidc::OidcProvider;
use crate::db::DbPool;
use crate::config::Config;

pub mod auth;
pub mod files;
pub mod admin;

/// Build the main router with all routes and middleware.
///
/// Layer ordering (innermost = closest to handler):
///   request -> auth_middleware -> Extension layers -> handler
/// auth_middleware is outermost so it can inject AuthCtx before
/// the Extension layers are visited by the inner call chain.
pub fn build_router(
    pool: DbPool,
    acl: AclChecker,
    config: Arc<Config>,
    oidc_provider: Option<OidcProvider>,
) -> Router {
    // CORS layer - allow credentials with explicit origin
    // Note: when allow_credentials is true, allow_headers cannot be Any (wildcard).
    // We explicitly list common headers instead.
    let cors = CorsLayer::new()
        .allow_credentials(true)
        .allow_origin(AllowOrigin::mirror_request())
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::HEAD,
            Method::OPTIONS,
        ])
        .allow_headers([
            HeaderName::from_static("content-type"),
            HeaderName::from_static("authorization"),
            HeaderName::from_static("accept"),
            HeaderName::from_static("cookie"),
            HeaderName::from_static("x-requested-with"),
        ]);

    // Health check (no auth)
    let health = Router::new()
        .route("/health", get(|| async { "ok" }));

    // Auth routes: login/oidc are public; me/logout require auth
    let auth_routes = Router::new()
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/me", get(auth::me))
        .route("/api/auth/oidc/login", get(auth::oidc_login))
        .route("/api/auth/oidc/callback", get(auth::oidc_callback))
        .layer(axum::middleware::from_fn(auth_middleware))
        .layer(Extension(pool.clone()))
        .layer(Extension(config.clone()))
        .layer(Extension(oidc_provider.clone()))
        .layer(cors.clone());

    // File routes: all require auth
    // Route ordering: more specific paths first to avoid conflicts
    let files_routes = Router::new()
        .route("/api/files/:path_id/browse/*sub_path", get(files::list_dir))
        .route("/api/files/:path_id/download/*filename", get(files::download).head(files::head_file))
        .route("/api/files/:path_id/mkdir", post(files::mkdir))
        .route("/api/files/:path_id/rename", post(files::rename))
        .route("/api/files/:path_id/upload", post(files::upload))
        .route("/api/files/:path_id/delete/:name", delete(files::delete))
        .route("/api/files/:path_id", get(files::list_dir_root))
        .route("/api/files", get(files::list_paths))
        .layer(axum::middleware::from_fn(auth_middleware))
        .layer(Extension(pool.clone()))
        .layer(Extension(acl.clone()))
        .layer(Extension(config.clone()))
        .layer(Extension(oidc_provider.clone()))
        .layer(cors.clone());

    // Admin routes: all require auth
    let admin_routes = Router::new()
        .route("/api/admin/users", get(admin::list_users))
        .route("/api/admin/users", post(admin::create_user))
        .route("/api/admin/users/:id", put(admin::update_user))
        .route("/api/admin/users/:id", delete(admin::delete_user))
        .route("/api/admin/groups", get(admin::list_groups))
        .route("/api/admin/groups", post(admin::create_group))
        .route("/api/admin/groups/:id", delete(admin::delete_group))
        .route("/api/admin/paths", get(admin::list_paths))
        .route("/api/admin/paths", post(admin::add_path))
        .route("/api/admin/paths/:id", delete(admin::remove_path))
        .route("/api/admin/permissions", get(admin::list_permissions))
        .route("/api/admin/permissions", post(admin::grant_permission))
        .route("/api/admin/permissions/:id", delete(admin::revoke_permission))
        .layer(axum::middleware::from_fn(auth_middleware))
        .layer(Extension(pool.clone()))
        .layer(Extension(acl.clone()))
        .layer(Extension(config.clone()))
        .layer(Extension(oidc_provider))
        .layer(cors.clone());

    // Merge all API routes; static files and SPA fallback are handled in main.rs.
    Router::new()
        .merge(health)
        .merge(auth_routes)
        .merge(files_routes)
        .merge(admin_routes)
}