# lid - Lightweight File Share Server

## 1. Project Overview

**Name:** lid (Lightweight IDrive alternative)  
**Type:** Self-hosted web file sharing server  
**Core:** Serve local directories via web with full user/group/ACL and OIDC support  
**Target:** Self-hosters who want Alist-like sharing without multi-drive complexity

---

## 2. Architecture

```
lid/
├── src/
│   ├── main.rs              # Entry point
│   ├── config.rs            # Config loading (config.yaml)
│   ├── db/
│   │   ├── mod.rs
│   │   ├── schema.rs        # SQLite schema init
│   │   └── models.rs        # User, Group, ACL, Path models
│   ├── auth/
│   │   ├── mod.rs
│   │   ├── oidc.rs          # OIDC provider integration
│   │   ├── jwt.rs           # JWT session tokens
│   │   └── password.rs      # Argon2 hashing
│   ├── acl/
│   │   ├── mod.rs
│   │   └── checker.rs       # Path permission checks
│   ├── api/
│   │   ├── mod.rs
│   │   ├── files.rs         # File listing/download/stream
│   │   ├── admin.rs         # Admin CRUD (users/groups/paths)
│   │   └── auth.rs          # Login/logout/session
│   ├── web/
│   │   ├── mod.rs
│   │   ├── serve.rs         # Static file server (SPA)
│   │   └── routes.rs        # Web UI routes
│   └── error.rs             # Unified error type
├── static/                  # Frontend SPA (built separately)
├── config.yaml              # Configuration file
├── SPEC.md
└── Cargo.toml
```

---

## 3. Data Model (SQLite)

### users
| Field | Type | Notes |
|-------|------|-------|
| id | TEXT (uuid) | PK |
| username | TEXT | unique |
| password_hash | TEXT | Argon2 |
| email | TEXT | unique |
| oidc_subject | TEXT | unique, nullable |
| is_admin | BOOL | superuser |
| created_at | INTEGER | unix timestamp |

### groups
| Field | Type | Notes |
|-------|------|-------|
| id | TEXT (uuid) | PK |
| name | TEXT | unique |

### group_members
| Field | Type | Notes |
|-------|------|-------|
| user_id | TEXT | FK → users.id |
| group_id | TEXT | FK → groups.id |

### paths
| Field | Type | Notes |
|-------|------|-------|
| id | TEXT (uuid) | PK |
| path | TEXT | absolute server path |
| display_name | TEXT | shown in UI |
| description | TEXT | optional |
| is_public | BOOL | accessible without login |

### permissions
| Field | Type | Notes |
|-------|------|-------|
| id | TEXT (uuid) | PK |
| principal_type | TEXT | "user" or "group" |
| principal_id | TEXT | user_id or group_id |
| path_id | TEXT | FK → paths.id |
| permission | TEXT | "read" / "write" / "admin" |

---

## 4. Authentication

### Local Auth
- Username + password (Argon2id)
- JWT stored in httpOnly cookie
- Session expiry: 7 days, sliding

### OIDC Auth
- Configurable provider (generic OIDC)
- Fields mapped: `sub` → `oidc_subject`, `email` → `email`, `name` → `username`
- Auto-provision users on first login
- Group sync from OIDC claims (configurable)

### Demo Mode
- Single admin account created on first run (from config)

---

## 5. ACL Model

**Permission levels (cumulative):**
- `read` — list directory, download files
- `write` — create/upload/rename/delete files
- `admin` — manage permissions on this path

**Resolution rules:**
1. Explicit user permission > group permission
2. Explicit group permission > inherited permission
3. No entry = deny
4. Public paths bypass auth for read (but audited)

---

## 6. API Endpoints

### Auth
- `POST /api/auth/login` — username/password → JWT
- `POST /api/auth/logout` — invalidate session
- `GET /api/auth/me` — current user info
- `GET /api/auth/oidc/login` — redirect to OIDC provider
- `GET /api/auth/oidc/callback` — OIDC callback

### Files (requires auth)
- `GET /api/files` — list root accessible paths
- `GET /api/files/{path_id}` — list directory contents
- `GET /api/files/{path_id}/download/{filename}` — download file
- `HEAD /api/files/{path_id}/download/{filename}` — check file exists
- `POST /api/files/{path_id}/upload` — upload file (chunked)
- `POST /api/files/{path_id}/mkdir` — create directory
- `DELETE /api/files/{path_id}/{name}` — delete file/dir
- `POST /api/files/{path_id}/rename` — rename

### Admin (requires admin)
- `GET /api/admin/users` — list users
- `POST /api/admin/users` — create user
- `PUT /api/admin/users/{id}` — update user
- `DELETE /api/admin/users/{id}` — delete user
- `GET /api/admin/groups` — list groups
- `POST /api/admin/groups` — create group
- `DELETE /api/admin/groups/{id}` — delete group
- `GET /api/admin/paths` — list managed paths
- `POST /api/admin/paths` — add path
- `DELETE /api/admin/paths/{id}` — remove path
- `GET /api/admin/permissions` — list permissions
- `POST /api/admin/permissions` — grant permission
- `DELETE /api/admin/permissions/{id}` — revoke

---

## 7. Frontend (SPA)

- **Framework:** Vanilla JS + minimal HTML (no build step for simplicity, or Preact for reactivity)
- **Pages:**
  - Login (`/login`) — username/password + OIDC button
  - File browser (`/`) — list paths user has access to, browse folders
  - Admin panel (`/admin`) — users, groups, paths, permissions management
- **Design:** Dark mode, clean, minimalist (inspired by Alist)

---

## 8. Configuration (config.yaml)

```yaml
server:
  host: "0.0.0.0"
  port: 8080
  base_url: "/"  # for reverse proxy

database:
  path: "./lid.db"

auth:
  jwt_secret: "change-me"
  session_days: 7
  
  # OIDC (optional)
  oidc:
    enabled: false
    provider_url: "https://your-idp.example.com"
    client_id: "lid"
    client_secret: "secret"
    scopes: ["openid", "email", "profile"]

demo:
  enabled: true
  username: "admin"
  password: "admin123"
```

---

## 9. Tech Stack

- **Language:** Rust (2021 edition)
- **Web framework:** Axum 0.7
- **Database:** SQLite via rusqlite + r2d2 connection pool
- **Auth:** jsonwebtoken, argon2
- **OIDC:** openidconnect
- **Async:** Tokio
- **Config:** serde_yaml
- **Logging:** tracing + tracing-subscriber

---

## 10. Development Plan

Phase 1: Core (this session)
- Project scaffolding + deps
- Config + DB schema
- Basic file serving (read-only)
- User model + local auth (JWT)
- ACL enforcement

Phase 2: Full API
- Complete CRUD API (files + admin)
- Web panel (SPA)
- OIDC integration

Phase 3: Polish
- Write/upload functionality
- Search, sorting
- Share links (public read access)
- Package as single binary