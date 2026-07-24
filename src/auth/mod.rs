pub mod jwt;
pub mod middleware;
pub mod oidc;
pub mod password;

use serde::{Deserialize, Serialize};

/// Represents the authenticated user context, injected by auth middleware.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCtx {
    pub user_id: String,
    pub username: String,
    pub is_admin: bool,
}
