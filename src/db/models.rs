use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::db::DbPool;
use crate::error::AppError;

// ── User ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub email: String,
    pub oidc_subject: Option<String>,
    pub is_admin: bool,
    pub created_at: i64,
}

impl User {
    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(User {
            id: row.get("id")?,
            username: row.get("username")?,
            password_hash: row.get("password_hash")?,
            email: row.get("email")?,
            oidc_subject: row.get("oidc_subject")?,
            is_admin: row.get::<_, i32>("is_admin")? != 0,
            created_at: row.get("created_at")?,
        })
    }

    pub fn find_by_id(conn: &Connection, id: &str) -> Result<Option<Self>, AppError> {
        let mut stmt = conn.prepare("SELECT * FROM users WHERE id = ?1")?;
        let mut rows = stmt.query_map(params![id], Self::from_row)?;
        Ok(rows.next().transpose()?)
    }

    pub fn find_by_username(conn: &Connection, username: &str) -> Result<Option<Self>, AppError> {
        let mut stmt = conn.prepare("SELECT * FROM users WHERE username = ?1")?;
        let mut rows = stmt.query_map(params![username], Self::from_row)?;
        Ok(rows.next().transpose()?)
    }

    pub fn find_by_email(conn: &Connection, email: &str) -> Result<Option<Self>, AppError> {
        let mut stmt = conn.prepare("SELECT * FROM users WHERE email = ?1")?;
        let mut rows = stmt.query_map(params![email], Self::from_row)?;
        Ok(rows.next().transpose()?)
    }

    pub fn find_by_oidc_subject(
        conn: &Connection,
        subject: &str,
    ) -> Result<Option<Self>, AppError> {
        let mut stmt = conn.prepare("SELECT * FROM users WHERE oidc_subject = ?1")?;
        let mut rows = stmt.query_map(params![subject], Self::from_row)?;
        Ok(rows.next().transpose()?)
    }

    pub fn count(conn: &Connection) -> Result<i64, AppError> {
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
        Ok(count)
    }

    pub fn list_all(conn: &Connection) -> Result<Vec<Self>, AppError> {
        let mut stmt = conn.prepare("SELECT * FROM users ORDER BY username")?;
        let rows = stmt.query_map([], Self::from_row)?;
        let mut users = Vec::new();
        for row in rows {
            users.push(row?);
        }
        Ok(users)
    }

    pub fn create(
        conn: &Connection,
        id: &str,
        username: &str,
        password_hash: &str,
        email: &str,
        oidc_subject: Option<&str>,
        is_admin: bool,
        created_at: i64,
    ) -> Result<(), AppError> {
        conn.execute(
            "INSERT INTO users (id, username, password_hash, email, oidc_subject, is_admin, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                username,
                password_hash,
                email,
                oidc_subject,
                is_admin as i32,
                created_at
            ],
        )?;
        Ok(())
    }

    pub fn update(
        conn: &Connection,
        id: &str,
        username: &str,
        email: &str,
        is_admin: bool,
    ) -> Result<(), AppError> {
        conn.execute(
            "UPDATE users SET username = ?2, email = ?3, is_admin = ?4 WHERE id = ?1",
            params![id, username, email, is_admin as i32],
        )?;
        Ok(())
    }

    pub fn update_password(
        conn: &Connection,
        id: &str,
        password_hash: &str,
    ) -> Result<(), AppError> {
        conn.execute(
            "UPDATE users SET password_hash = ?2 WHERE id = ?1",
            params![id, password_hash],
        )?;
        Ok(())
    }

    pub fn delete(conn: &Connection, id: &str) -> Result<(), AppError> {
        conn.execute("DELETE FROM users WHERE id = ?1", params![id])?;
        Ok(())
    }
}

// ── Group ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
}

impl Group {
    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Group {
            id: row.get("id")?,
            name: row.get("name")?,
        })
    }

    pub fn find_by_id(conn: &Connection, id: &str) -> Result<Option<Self>, AppError> {
        let mut stmt = conn.prepare("SELECT * FROM groups WHERE id = ?1")?;
        let mut rows = stmt.query_map(params![id], Self::from_row)?;
        Ok(rows.next().transpose()?)
    }

    pub fn list_all(conn: &Connection) -> Result<Vec<Self>, AppError> {
        let mut stmt = conn.prepare("SELECT * FROM groups ORDER BY name")?;
        let rows = stmt.query_map([], Self::from_row)?;
        let mut groups = Vec::new();
        for row in rows {
            groups.push(row?);
        }
        Ok(groups)
    }

    pub fn create(conn: &Connection, id: &str, name: &str) -> Result<(), AppError> {
        conn.execute(
            "INSERT INTO groups (id, name) VALUES (?1, ?2)",
            params![id, name],
        )?;
        Ok(())
    }

    pub fn delete(conn: &Connection, id: &str) -> Result<(), AppError> {
        conn.execute("DELETE FROM groups WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_user_groups(conn: &Connection, user_id: &str) -> Result<Vec<Self>, AppError> {
        let mut stmt = conn.prepare(
            "SELECT g.* FROM groups g
             INNER JOIN group_members gm ON g.id = gm.group_id
             WHERE gm.user_id = ?1
             ORDER BY g.name",
        )?;
        let rows = stmt.query_map(params![user_id], Self::from_row)?;
        let mut groups = Vec::new();
        for row in rows {
            groups.push(row?);
        }
        Ok(groups)
    }
}

// ── GroupMember ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    pub user_id: String,
    pub group_id: String,
}

impl GroupMember {
    pub fn add(conn: &Connection, user_id: &str, group_id: &str) -> Result<(), AppError> {
        conn.execute(
            "INSERT OR IGNORE INTO group_members (user_id, group_id) VALUES (?1, ?2)",
            params![user_id, group_id],
        )?;
        Ok(())
    }

    pub fn remove(conn: &Connection, user_id: &str, group_id: &str) -> Result<(), AppError> {
        conn.execute(
            "DELETE FROM group_members WHERE user_id = ?1 AND group_id = ?2",
            params![user_id, group_id],
        )?;
        Ok(())
    }
}

// ── PathEntry ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathEntry {
    pub id: String,
    pub path: String,
    pub display_name: String,
    pub description: String,
    pub is_public: bool,
}

impl PathEntry {
    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(PathEntry {
            id: row.get("id")?,
            path: row.get("path")?,
            display_name: row.get("display_name")?,
            description: row.get("description")?,
            is_public: row.get::<_, i32>("is_public")? != 0,
        })
    }

    pub fn find_by_id(conn: &Connection, id: &str) -> Result<Option<Self>, AppError> {
        let mut stmt = conn.prepare("SELECT * FROM paths WHERE id = ?1")?;
        let mut rows = stmt.query_map(params![id], Self::from_row)?;
        Ok(rows.next().transpose()?)
    }

    pub fn list_all(conn: &Connection) -> Result<Vec<Self>, AppError> {
        let mut stmt = conn.prepare("SELECT * FROM paths ORDER BY display_name")?;
        let rows = stmt.query_map([], Self::from_row)?;
        let mut paths = Vec::new();
        for row in rows {
            paths.push(row?);
        }
        Ok(paths)
    }

    pub fn list_public(conn: &Connection) -> Result<Vec<Self>, AppError> {
        let mut stmt = conn.prepare("SELECT * FROM paths WHERE is_public = 1 ORDER BY display_name")?;
        let rows = stmt.query_map([], Self::from_row)?;
        let mut paths = Vec::new();
        for row in rows {
            paths.push(row?);
        }
        Ok(paths)
    }

    pub fn create(
        conn: &Connection,
        id: &str,
        path: &str,
        display_name: &str,
        description: &str,
        is_public: bool,
    ) -> Result<(), AppError> {
        conn.execute(
            "INSERT INTO paths (id, path, display_name, description, is_public)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, path, display_name, description, is_public as i32],
        )?;
        Ok(())
    }

    pub fn delete(conn: &Connection, id: &str) -> Result<(), AppError> {
        conn.execute("DELETE FROM paths WHERE id = ?1", params![id])?;
        Ok(())
    }
}

// ── Permission ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub id: String,
    pub principal_type: String,
    pub principal_id: String,
    pub path_id: String,
    pub permission: String,
}

impl Permission {
    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Permission {
            id: row.get("id")?,
            principal_type: row.get("principal_type")?,
            principal_id: row.get("principal_id")?,
            path_id: row.get("path_id")?,
            permission: row.get("permission")?,
        })
    }

    pub fn find_by_id(conn: &Connection, id: &str) -> Result<Option<Self>, AppError> {
        let mut stmt = conn.prepare("SELECT * FROM permissions WHERE id = ?1")?;
        let mut rows = stmt.query_map(params![id], Self::from_row)?;
        Ok(rows.next().transpose()?)
    }

    pub fn list_all(conn: &Connection) -> Result<Vec<Self>, AppError> {
        let mut stmt = conn.prepare("SELECT * FROM permissions")?;
        let rows = stmt.query_map([], Self::from_row)?;
        let mut perms = Vec::new();
        for row in rows {
            perms.push(row?);
        }
        Ok(perms)
    }

    pub fn find_for_user_and_path(
        conn: &Connection,
        user_id: &str,
        path_id: &str,
    ) -> Result<Vec<Self>, AppError> {
        let mut stmt = conn.prepare(
            "SELECT * FROM permissions
             WHERE path_id = ?1
             AND ((principal_type = 'user' AND principal_id = ?2)
                  OR (principal_type = 'group' AND principal_id IN
                      (SELECT group_id FROM group_members WHERE user_id = ?2)))",
        )?;
        let rows = stmt.query_map(params![path_id, user_id], Self::from_row)?;
        let mut perms = Vec::new();
        for row in rows {
            perms.push(row?);
        }
        Ok(perms)
    }

    pub fn find_paths_for_user(conn: &Connection, user_id: &str) -> Result<Vec<String>, AppError> {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT path_id FROM permissions
             WHERE (principal_type = 'user' AND principal_id = ?1)
                OR (principal_type = 'group' AND principal_id IN
                    (SELECT group_id FROM group_members WHERE user_id = ?1))",
        )?;
        let rows = stmt.query_map(params![user_id], |row| row.get::<_, String>(0))?;
        let mut path_ids = Vec::new();
        for row in rows {
            path_ids.push(row?);
        }
        Ok(path_ids)
    }

    pub fn create(
        conn: &Connection,
        id: &str,
        principal_type: &str,
        principal_id: &str,
        path_id: &str,
        permission: &str,
    ) -> Result<(), AppError> {
        conn.execute(
            "INSERT INTO permissions (id, principal_type, principal_id, path_id, permission)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, principal_type, principal_id, path_id, permission],
        )?;
        Ok(())
    }

    pub fn delete(conn: &Connection, id: &str) -> Result<(), AppError> {
        conn.execute("DELETE FROM permissions WHERE id = ?1", params![id])?;
        Ok(())
    }
}
