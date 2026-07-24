# Task: Business Logic Review & Fix

## Objective
Review ALL Rust source files in this project for:
1. Business logic correctness - can the code actually run end-to-end?
2. Logic bugs (wrong conditions, missing checks, incorrect flow)
3. Non-production security issues (e.g. hardcoded secrets, missing auth checks, unvalidated user input that causes logic errors)
   - NOTE: production-grade security hardening (rate limiting, HTTPS enforcement, etc.) is NOT required

## Source Files to Review
- src/api/auth.rs
- src/api/files.rs
- src/api/mod.rs
- src/api/admin.rs
- src/acl/checker.rs
- src/acl/mod.rs
- src/auth/jwt.rs
- src/auth/middleware.rs
- src/auth/mod.rs
- src/auth/oidc.rs
- src/auth/password.rs
- src/config.rs
- src/db/mod.rs
- src/db/models.rs
- src/error.rs
- src/main.rs
- src/web/mod.rs

## Instructions
1. Read EVERY source file listed above completely
2. Understand the overall architecture (Rust web app with JWT auth + file management + ACL)
3. Identify ALL bugs: logic errors, wrong flow, missing error handling, unvalidated inputs causing crashes
4. Fix ALL identified bugs directly in the source files
5. After fixing, verify the code compiles: run `cargo check 2>&1`
6. If cargo check passes, run `cargo build 2>&1` to confirm full build
7. Commit all changes: `git add -A && git commit -m "fix: business logic review and bug fixes"`
8. Push to origin agent branch: `git push origin agent`

## Model Configuration
Use DeepSeek V4 Pro via OpenAI-compatible API:
- Base URL: https://api.deepseek.com
- API Key: (redacted - use environment variable)
- Model: deepseek-chat (DeepSeek-V3 / V4 Pro)

Set these environment variables before running:
```
OPENCODE_API_URL=https://api.deepseek.com
OPENCODE_API_KEY=<your-api-key>
OPENCODE_MODEL=deepseek-chat
```

## Output Required
After completion, provide a summary of:
- All bugs found (file, line, description)
- All fixes applied
- Compilation status
- Git push status
