use crate::db::DbPool;
use crate::db::models::{PathEntry, Permission};
use crate::error::AppError;

/// ACL permission checker.
///
/// Resolution rules:
/// 1. Explicit user permission > group permission
/// 2. No entry = deny
/// 3. Public paths bypass auth for read
#[derive(Clone)]
pub struct AclChecker {
    pub pool: DbPool,
}

impl AclChecker {
    pub fn new(pool: DbPool) -> Self {
        AclChecker { pool }
    }

    /// Check if user has the given permission on a path.
    /// Permission levels: "read" < "write" < "admin" (cumulative).
    pub fn check(
        &self,
        user_id: &str,
        path_id: &str,
        required: &str,
    ) -> Result<bool, AppError> {
        let conn = self.pool.get()?;

        // Check if path is public and only read is required
        if required == "read" {
            if let Some(path_entry) = PathEntry::find_by_id(&conn, path_id)? {
                if path_entry.is_public {
                    return Ok(true);
                }
            }
        }

        let perms = Permission::find_for_user_and_path(&conn, user_id, path_id)?;

        if perms.is_empty() {
            return Ok(false);
        }

        // Check user-level permissions first (higher priority)
        let user_perms: Vec<&Permission> = perms
            .iter()
            .filter(|p| p.principal_type == "user")
            .collect();

        if !user_perms.is_empty() {
            return Ok(user_perms.iter().any(|p| permission_satisfies(&p.permission, required)));
        }

        // Fall back to group permissions
        let group_perms: Vec<&Permission> = perms
            .iter()
            .filter(|p| p.principal_type == "group")
            .collect();

        Ok(group_perms.iter().any(|p| permission_satisfies(&p.permission, required)))
    }

    /// List all paths accessible to the user (including public).
    pub fn list_accessible_paths(&self, user_id: &str) -> Result<Vec<PathEntry>, AppError> {
        let conn = self.pool.get()?;

        let mut accessible = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        // Get paths through permissions
        let path_ids = Permission::find_paths_for_user(&conn, user_id)?;
        for path_id in &path_ids {
            if let Some(entry) = PathEntry::find_by_id(&conn, path_id)? {
                seen_ids.insert(entry.id.clone());
                accessible.push(entry);
            }
        }

        // Add public paths
        let public_paths = PathEntry::list_public(&conn)?;
        for entry in public_paths {
            if !seen_ids.contains(&entry.id) {
                accessible.push(entry);
            }
        }

        accessible.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        Ok(accessible)
    }
}

/// Check if a granted permission level satisfies a required level.
/// "admin" satisfies everything, "write" satisfies "write" and "read",
/// "read" only satisfies "read".
fn permission_satisfies(granted: &str, required: &str) -> bool {
    match granted {
        "admin" => true,
        "write" => required == "write" || required == "read",
        "read" => required == "read",
        _ => false,
    }
}
