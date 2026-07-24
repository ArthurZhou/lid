# lid Bug Fix Task

Analyze and fix bugs in the lid web file sharing server at ~/dev/openclaw/workspace/lid

## Current State
- `cargo build --release` succeeds with 16 warnings but 0 errors
- Code compiles but has runtime/logic issues

## Bugs to Fix

### B1: OIDC Provider is always None
**File**: src/api/mod.rs
**Problem**: All routers pass `Extension(None as Option<OidcProvider>)`. The OIDC provider should be initialized from config when `config.auth.oidc.enabled` is true.
**Fix**: Initialize OidcProvider in `build_router()` when OIDC is enabled, pass it to all routes that need it.

### B2: Route conflict in files_routes
**File**: src/api/mod.rs  
**Problem**: Routes `GET /api/files/:path_id` (list_dir_root) and `DELETE /api/files/:path_id/:name` (delete) — the `:name` wildcard can match "browse" and other path segments incorrectly.
**Fix**: Restructure routes so the static segment `browse` is matched before the wildcard:
- `/api/files/:path_id/browse/*sub_path` (GET list_dir) - already correct
- `/api/files/:path_id/download/*filename` (GET download) - already correct  
- `/api/files/:path_id` (GET list_dir_root) - this needs to be after browse/*sub_path to avoid matching "browse"
- Actually the real issue: DELETE `/:path_id/:name` conflicts with GET `/:path_id` for different methods. In Axum, GET and DELETE on the same path should be fine. But the wildcard `/:name` could incorrectly capture path segments like "browse" or "download". Move delete to a more explicit path or add a guard.

### B3: Missing CORS middleware
**Files**: src/api/mod.rs, src/main.rs
**Problem**: No CORS headers are set. Browser requests from the frontend (especially in dev with different ports) will be blocked.
**Fix**: Add CORS middleware to all routes. Use tower-http's CorsLayer.

### B4: login handler returns both token and cookie (redundant)
**File**: src/api/auth.rs
**Problem**: The login response includes both `token` in the JSON body AND sets a `lid_token` cookie. The token in JSON body is redundant since the frontend can read it from the cookie. But more importantly: the frontend SPA needs the token in JSON for cases where cookie is not handled (e.g., curl). Keep the cookie but also return the token in JSON.
**This is actually fine, not a bug.**

### B5: SPA fallback at root level catches /api/* routes  
**File**: src/api/mod.rs
**Problem**: The `spa_fallback` router has a `.fallback()` that catches ALL unmatched routes. When merged with auth_routes/files_routes/admin_routes at root level, there could be route resolution ambiguity.
**Fix**: Put the fallback only on the web router, not mixed with API routes. Make sure API routes take precedence.

## Verification
After each fix, run: `cd ~/dev/openclaw/workspace/lid && cargo build --release 2>&1`
Final: `cd ~/dev/openclaw/workspace/lid && cargo build --release` must succeed with no errors (warnings OK).