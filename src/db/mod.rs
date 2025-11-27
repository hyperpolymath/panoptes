// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

//! Database module for file metadata, tags, and categories

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::{PanoptesError, Result};

/// Database manager for Panoptes (thread-safe wrapper)
#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

/// A processed file record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub id: String,
    pub original_path: String,
    pub new_path: String,
    pub suggested_name: String,
    pub file_hash: String,
    pub category: Option<String>,
    pub confidence: f64,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// A tag
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub category: Option<String>,
}

/// A category with statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub name: String,
    pub description: Option<String>,
    pub file_count: i64,
}

/// Database statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbStats {
    pub file_count: i64,
    pub tag_count: i64,
    pub category_count: i64,
}

impl Database {
    /// Open or create the database
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.initialize()?;
        Ok(db)
    }

    /// Open an in-memory database (for testing)
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.initialize()?;
        Ok(db)
    }

    fn lock_conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.conn.lock().map_err(|_| PanoptesError::Config("Database lock poisoned".to_string()))
    }

    /// Initialize database schema
    fn initialize(&self) -> Result<()> {
        let conn = self.lock_conn()?;
        conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS files (
                id TEXT PRIMARY KEY,
                original_path TEXT NOT NULL,
                suggested_name TEXT NOT NULL,
                file_hash TEXT NOT NULL,
                category TEXT,
                confidence REAL DEFAULT 0.0,
                metadata TEXT DEFAULT '{}',
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                category TEXT,
                UNIQUE(name, category)
            );

            CREATE TABLE IF NOT EXISTS file_tags (
                file_id TEXT NOT NULL,
                tag_id INTEGER NOT NULL,
                PRIMARY KEY (file_id, tag_id)
            );

            CREATE INDEX IF NOT EXISTS idx_files_hash ON files(file_hash);
            CREATE INDEX IF NOT EXISTS idx_files_category ON files(category);
        "#)?;
        Ok(())
    }

    /// Insert a new file record
    pub fn insert_file(
        &self,
        id: &str,
        original_path: &str,
        suggested_name: &str,
        file_hash: &str,
        category: Option<&str>,
        confidence: f64,
        metadata: &serde_json::Value,
    ) -> Result<()> {
        let conn = self.lock_conn()?;
        let metadata_json = serde_json::to_string(metadata)?;

        conn.execute(
            r#"INSERT OR REPLACE INTO files (id, original_path, suggested_name, file_hash, category, confidence, metadata, created_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'))"#,
            params![id, original_path, suggested_name, file_hash, category, confidence, metadata_json],
        )?;
        Ok(())
    }

    /// Add a tag
    pub fn add_tag(&self, file_id: &str, tag_name: &str, category: Option<&str>) -> Result<()> {
        let conn = self.lock_conn()?;

        // Insert tag if not exists
        conn.execute(
            "INSERT OR IGNORE INTO tags (name, category) VALUES (?1, ?2)",
            params![tag_name, category],
        )?;

        // Get tag id
        let tag_id: i64 = conn.query_row(
            "SELECT id FROM tags WHERE name = ?1",
            params![tag_name],
            |row| row.get(0),
        )?;

        // Link to file
        conn.execute(
            "INSERT OR IGNORE INTO file_tags (file_id, tag_id) VALUES (?1, ?2)",
            params![file_id, tag_id],
        )?;

        Ok(())
    }

    /// Get all tags
    pub fn get_all_tags(&self) -> Result<Vec<Tag>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT id, name, category FROM tags ORDER BY name")?;
        let tags = stmt.query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                category: row.get(2)?,
            })
        })?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(tags)
    }

    /// Get all categories with counts
    pub fn get_all_categories(&self) -> Result<Vec<Category>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            r#"SELECT COALESCE(category, 'Uncategorized') as cat, COUNT(*) as cnt
               FROM files GROUP BY category ORDER BY cnt DESC"#
        )?;
        let cats = stmt.query_map([], |row| {
            Ok(Category {
                name: row.get(0)?,
                description: None,
                file_count: row.get(1)?,
            })
        })?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(cats)
    }

    /// Search files
    pub fn search_files(&self, query: &str, limit: usize) -> Result<Vec<FileRecord>> {
        let conn = self.lock_conn()?;
        let pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(
            r#"SELECT id, original_path, suggested_name, file_hash, category, confidence, metadata, created_at
               FROM files WHERE suggested_name LIKE ?1 OR original_path LIKE ?1
               ORDER BY created_at DESC LIMIT ?2"#
        )?;

        let files = stmt.query_map(params![pattern, limit as i64], |row| {
            let metadata_str: String = row.get(6)?;
            let created_str: String = row.get(7)?;
            Ok(FileRecord {
                id: row.get(0)?,
                original_path: row.get(1)?,
                new_path: row.get(1)?,
                suggested_name: row.get(2)?,
                file_hash: row.get(3)?,
                category: row.get(4)?,
                confidence: row.get(5)?,
                metadata: serde_json::from_str(&metadata_str).unwrap_or(serde_json::json!({})),
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(files)
    }

    /// Get all files
    pub fn get_all_files(&self) -> Result<Vec<FileRecord>> {
        self.search_files("", 1000)
    }

    /// Get database statistics
    pub fn get_stats(&self) -> Result<DbStats> {
        let conn = self.lock_conn()?;
        let file_count: i64 = conn.query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))?;
        let tag_count: i64 = conn.query_row("SELECT COUNT(*) FROM tags", [], |row| row.get(0))?;
        let category_count: i64 = conn.query_row(
            "SELECT COUNT(DISTINCT category) FROM files WHERE category IS NOT NULL",
            [],
            |row| row.get(0),
        )?;
        Ok(DbStats { file_count, tag_count, category_count })
    }

    /// Vacuum database
    pub fn vacuum(&self) -> Result<()> {
        let conn = self.lock_conn()?;
        conn.execute("VACUUM", [])?;
        Ok(())
    }

    /// Check for duplicate by hash
    pub fn find_duplicate(&self, hash: &str) -> Result<Option<String>> {
        let conn = self.lock_conn()?;
        let result: rusqlite::Result<String> = conn.query_row(
            "SELECT id FROM files WHERE file_hash = ?1 LIMIT 1",
            params![hash],
            |row| row.get(0),
        );
        match result {
            Ok(id) => Ok(Some(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    // Methods for web UI compatibility
    pub fn get_recent_files(&self, limit: usize) -> Result<Vec<FileRecord>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            r#"SELECT id, original_path, suggested_name, file_hash, category, confidence, metadata, created_at
               FROM files ORDER BY created_at DESC LIMIT ?1"#
        )?;

        let files = stmt.query_map(params![limit as i64], |row| {
            let metadata_str: String = row.get(6)?;
            let created_str: String = row.get(7)?;
            Ok(FileRecord {
                id: row.get(0)?,
                original_path: row.get(1)?,
                new_path: row.get(1)?,
                suggested_name: row.get(2)?,
                file_hash: row.get(3)?,
                category: row.get(4)?,
                confidence: row.get(5)?,
                metadata: serde_json::from_str(&metadata_str).unwrap_or(serde_json::json!({})),
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(files)
    }

    pub fn get_category_stats(&self) -> Result<Vec<(String, i64)>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            r#"SELECT COALESCE(category, 'Uncategorized'), COUNT(*) FROM files GROUP BY category"#
        )?;
        let stats = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(stats)
    }

    pub fn get_file_count(&self) -> Result<i64> {
        let conn = self.lock_conn()?;
        conn.query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))
            .map_err(Into::into)
    }

    pub fn get_files_by_category(&self, category: &str, limit: usize) -> Result<Vec<FileRecord>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            r#"SELECT id, original_path, suggested_name, file_hash, category, confidence, metadata, created_at
               FROM files WHERE category = ?1 ORDER BY created_at DESC LIMIT ?2"#
        )?;

        let files = stmt.query_map(params![category, limit as i64], |row| {
            let metadata_str: String = row.get(6)?;
            let created_str: String = row.get(7)?;
            Ok(FileRecord {
                id: row.get(0)?,
                original_path: row.get(1)?,
                new_path: row.get(1)?,
                suggested_name: row.get(2)?,
                file_hash: row.get(3)?,
                category: row.get(4)?,
                confidence: row.get(5)?,
                metadata: serde_json::from_str(&metadata_str).unwrap_or(serde_json::json!({})),
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(files)
    }

    pub fn add_tag_to_file(&self, file_id: &str, tag_name: &str) -> Result<()> {
        self.add_tag(file_id, tag_name, None)
    }

    pub fn remove_tag_from_file(&self, file_id: &str, tag_name: &str) -> Result<()> {
        let conn = self.lock_conn()?;
        conn.execute(
            r#"DELETE FROM file_tags WHERE file_id = ?1
               AND tag_id = (SELECT id FROM tags WHERE name = ?2)"#,
            params![file_id, tag_name],
        )?;
        Ok(())
    }
}

/// Generate a new UUID for file records
pub fn new_file_id() -> String {
    Uuid::new_v4().to_string()
}
