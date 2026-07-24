mod config;
mod db;
mod error;
mod acl;
mod auth;
mod api;
mod web;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use axum::Router;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::db::DbPool;
use crate::db::models::User;
use crate::acl::checker::AclChecker;
use crate::auth::password;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Init tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load config
    let cfg = config::load()?;

    tracing::info!("Starting lid server on {}:{}", cfg.server.host, cfg.server.port);

    // Init DB
    let pool = db::init_db(&cfg.database)?;

    // Create demo user if configured and no users exist
    if cfg.demo.enabled {
        let conn = pool.get()?;
        if User::count(&conn)? == 0 {
            tracing::info!("Creating demo user: {}", cfg.demo.username);
            let hash = password::hash_password(&cfg.demo.password)
                .map_err(|e| format!("Password hashing failed: {}", e))?;
            let now = chrono::Utc::now().timestamp();
            User::create(
                &conn,
                &uuid::Uuid::new_v4().to_string(),
                &cfg.demo.username,
                &hash,
                &format!("{}@localhost", cfg.demo.username),
                None,
                true,
                now,
            )?;
        }
    }

    // Create ACL checker
    let acl = AclChecker::new(pool.clone());

    // Build API router with real config (applies auth middleware internally)
    let api_router = api::build_router(pool.clone(), acl, Arc::new(cfg.clone()));

    // Build web router for static files (no auth needed)
    let static_dir = PathBuf::from("static");
    let web_router = web::build_web_router(static_dir);

    // Merge: API routes at /api, web at /*
    let app = api_router.merge(web_router).layer(TraceLayer::new_for_http());

    let addr: SocketAddr = format!("{}:{}", cfg.server.host, cfg.server.port)
        .parse()
        .map_err(|e| format!("Invalid address: {}", e))?;

    tracing::info!("lid listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}