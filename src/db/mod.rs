pub mod models;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use crate::config::DatabaseConfig;

pub type DbPool = Pool<SqliteConnectionManager>;

pub fn init_db(config: &DatabaseConfig) -> Result<DbPool, Box<dyn std::error::Error>> {
    let manager = SqliteConnectionManager::file(&config.path);
    let pool = Pool::builder().max_size(10).build(manager)?;

    // Run migrations
    let conn = pool.get()?;

    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL DEFAULT '',
            email TEXT NOT NULL DEFAULT '' UNIQUE,
            oidc_subject TEXT UNIQUE,
            is_admin INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS groups (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE
        );

        CREATE TABLE IF NOT EXISTS group_members (
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            group_id TEXT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
            PRIMARY KEY (user_id, group_id)
        );

        CREATE TABLE IF NOT EXISTS paths (
            id TEXT PRIMARY KEY,
            path TEXT NOT NULL,
            display_name TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            is_public INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS permissions (
            id TEXT PRIMARY KEY,
            principal_type TEXT NOT NULL,
            principal_id TEXT NOT NULL,
            path_id TEXT NOT NULL REFERENCES paths(id) ON DELETE CASCADE,
            permission TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_permissions_principal
            ON permissions(principal_type, principal_id);
        CREATE INDEX IF NOT EXISTS idx_permissions_path
            ON permissions(path_id);
        CREATE INDEX IF NOT EXISTS idx_group_members_user
            ON group_members(user_id);
        CREATE INDEX IF NOT EXISTS idx_group_members_group
            ON group_members(group_id);
        ",
    )?;

    tracing::info!("Database initialized at {}", config.path);
    Ok(pool)
}
