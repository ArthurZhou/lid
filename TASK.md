# lid - Web File Share Server Implementation

## Context

Project at: `/Users/zhousongtao/dev/openclaw/workspace/lid`
Already exists: empty Cargo.toml (Rust), git repo initialized, SPEC.md written.

## Task

Build a complete web file sharing server in Rust. Follow SPEC.md exactly.

## Phase 1: Project Setup

Update `Cargo.toml`:
```toml
[package]
name = "lid"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["fs", "trace", "cors"] }
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
serde_json = "1"
rusqlite = { version = "0.31", features = ["bundled"] }
r2d2 = "0.8"
r2d2_sqlite = "0.24"
argon2 = "0.5"
jsonwebtoken = "9"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
openidconnect = "4"
reqwest = { version = "0.12", features = ["json"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
thiserror = "1"
tower-layers = "0.12"
axum-extra = { version = "0.9", features = ["cookie"] }
cookie = "0.18"
async-trait = "0.1"
futures = "0.3"
mime = "0.3"
mime_guess = "2"
walkdir = "2"
sha2 = "0.10"
hex = "0.4"

[dev-dependencies]
tempfile = "3"
```

## Phase 2: Source Files

Create the following file structure. Each file must be complete and production-ready.

### `src/error.rs`
Unified `AppError` enum with `IntoResponse`. Variants: `NotFound`, `Unauthorized`, `Forbidden`, `BadRequest`, `InternalError`, `DatabaseError`, `IoError`. Implement `From<rusqlite::Error>`, `From<std::io::Error>`, `From<jsonwebtoken::Error>`.

### `src/config.rs`
Load `config.yaml` from current dir. Structs: `Config`, `ServerConfig`, `DatabaseConfig`, `AuthConfig`, `OidcConfig`, `DemoConfig`. Provide `load()` that tries `./config.yaml` then `/etc/lid/config.yaml`.

### `src/db/mod.rs`
Init SQLite, run migrations (CREATE TABLE IF NOT EXISTS). Expose `DbPool` as r2d2 pool wrapper. Functions: `init_db(config: &DatabaseConfig) -> DbPool`.

### `src/db/models.rs`
SQLx-free models using rusqlite. Structs: `User`, `Group`, `PathEntry`, `Permission`. Each with `find_by_id`, `find_by_username`, `find_by_email`, `create`, `update`, `delete` as associated functions. `GroupMember` for membership. All using `rusqlite::Connection` + `DbPool`.

### `src/auth/mod.rs`
Auth module entry. Re-export `AuthCtx` (logged-in user context).

### `src/auth/password.rs`
`hash_password()` and `verify_password()` using argon2. `generate盐()`.

### `src/auth/jwt.rs`
`Claims` struct (sub: user_id, exp, iat). `create_token()` and `verify_token()` using JWT HS256. Token expiry from config.

### `src/auth/oidc.rs`
`OidcProvider` struct. `discover()` from `.well-known/openid-configuration`. `exchange_code()` for token exchange. `fetch_userinfo()`. Return `OidcUser { subject, email, name }`.

### `src/auth/middleware.rs`
Axum middleware layer. Extract JWT from `Cookie` header or `Authorization: Bearer`. Validate, look up user, inject `AuthCtx` into request extensions. Skip auth for: `/api/auth/login`, `/api/auth/oidc/*`, `/static/*`, `/api/files/public/*`.

### `src/acl/mod.rs`
ACL module entry.

### `src/acl/checker.rs`
`AclChecker` struct holding `DbPool`. Methods:
- `check(user_id, path_id, permission) -> bool`
- `list_accessible_paths(user_id) -> Vec<PathEntry>`
- Permission resolution: explicit user > group > inherited, no entry = deny

### `src/api/mod.rs`
Router builder. Mount sub-routers: `auth_routes`, `files_routes`, `admin_routes`. Apply auth middleware.

### `src/api/auth.rs`
Handlers:
- `login` - POST /api/auth/login {username, password} → JWT cookie + user info
- `logout` - POST /api/auth/logout → clear cookie
- `me` - GET /api/auth/me → current user
- `oidc_login` - GET /api/auth/oidc/login → redirect to IdP
- `oidc_callback` - GET /api/auth/oidc/callback {code} → exchange, provision user, JWT cookie

### `src/api/files.rs`
Handlers for file operations. All take `path_id` as path param, extract from URL.
- `list_paths` - GET /api/files → user's accessible paths
- `list_dir` - GET /api/files/{path_id} → list directory contents (name, size, modified, is_dir)
- `download` - GET /api/files/{path_id}/download/{filename} → stream file with proper Content-Disposition
- `head_file` - HEAD /api/files/{path_id}/download/{filename}
- `mkdir` - POST /api/files/{path_id}/mkdir {name}
- `delete` - DELETE /api/files/{path_id}/{name}
- `rename` - POST /api/files/{path_id}/rename {old_name, new_name}
- `upload` - POST /api/files/{path_id}/upload (multipart form)

Use `walkdir::WalkDir` for directory listing. Stream downloads with `axum::body::StreamBody`. Check ACL `read` for listing/download, `write` for mkdir/delete/rename/upload.

### `src/api/admin.rs`
Handlers (all require `is_admin`):
- `list_users`, `create_user`, `update_user`, `delete_user`
- `list_groups`, `create_group`, `delete_group`
- `list_paths`, `add_path`, `remove_path`
- `list_permissions`, `grant_permission`, `revoke_permission`

### `src/web/mod.rs`
Serve the frontend SPA. Mount `static/` directory at `/`. Fallback to `index.html` for SPA routing.

### `src/main.rs`
1. Load config
2. Init DB
3. Create demo user if configured and no users exist
4. Build Axum router with all routes
5. Start server on `host:port`

## Phase 3: Frontend

Create `static/` directory. Write `static/index.html` (login page + main app all-in-one for simplicity, no build step).

The SPA should:
- Have a login page (username/password + "Login with OIDC" button if OIDC enabled)
- After login show file browser (sidebar with accessible paths, main area with file list)
- Admin panel accessible at `/admin` for admin users
- Dark theme, clean Alist-inspired style
- Use `fetch()` API calls to `/api/*`
- Handle JWT cookie automatically (credentials: 'include')

## Phase 4: Config file

Create `config.yaml`:
```yaml
server:
  host: "0.0.0.0"
  port: 8080
  base_url: "/"

database:
  path: "./lid.db"

auth:
  jwt_secret: "change-me-in-production-32chars!"
  session_days: 7

  oidc:
    enabled: false
    provider_url: ""
    client_id: ""
    client_secret: ""
    scopes: ["openid", "email", "profile"]

demo:
  enabled: true
  username: "admin"
  password: "admin123"
```

## Important Notes

1. Use only `rusqlite` directly, NOT sqlx (no compile-time query checking to avoid complexity)
2. All timestamps as Unix integers
3. All UUIDs as text strings
4. File paths stored as absolute server paths
5. Directory listing: sort dirs first, then files, by name
6. Serve correct MIME types using `mime_guess`
7. CORS: allow credentials, same-origin for simplicity
8. All API responses: JSON `{ "data": ... }` or `{ "error": "message" }`
9. Handle errors gracefully - no panics, proper HTTP status codes

## Verification

After implementation:
1. `cd /Users/zhousongtao/dev/openclaw/workspace/lid && cargo build --release` must succeed with no errors
2. Run the binary - it should start and create the DB
3. Try logging in with admin/admin123
4. Check all routes respond correctly