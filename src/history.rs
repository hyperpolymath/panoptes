// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

//! History management for undo support

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use crate::Result;

/// A single rename operation in history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub original_path: PathBuf,
    pub new_path: PathBuf,
    pub ai_suggestion: String,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub file_hash: String,
    pub undone: bool,
}

/// History manager for tracking file renames
pub struct History {
    path: PathBuf,
}

impl History {
    /// Create a new history manager
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Append an entry to the history
    pub fn append(&self, entry: &HistoryEntry) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        let json = serde_json::to_string(entry)?;
        writeln!(file, "{}", json)?;

        Ok(())
    }

    /// Read all history entries
    pub fn read_all(&self) -> Result<Vec<HistoryEntry>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);

        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str(&line) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    tracing::warn!("Failed to parse history entry: {}", e);
                }
            }
        }

        Ok(entries)
    }

    /// Get the most recent N entries (newest first)
    pub fn get_recent(&self, count: usize) -> Result<Vec<HistoryEntry>> {
        let mut entries = self.read_all()?;
        entries.reverse();
        entries.truncate(count);
        Ok(entries)
    }

    /// Mark an entry as undone
    pub fn mark_undone(&self, id: &str) -> Result<()> {
        let entries = self.read_all()?;

        // Rewrite the entire file with the updated entry
        let file = File::create(&self.path)?;
        let mut writer = std::io::BufWriter::new(file);

        for mut entry in entries {
            if entry.id == id {
                entry.undone = true;
            }
            let json = serde_json::to_string(&entry)?;
            writeln!(writer, "{}", json)?;
        }

        Ok(())
    }

    /// Get entries that haven't been undone
    pub fn get_undoable(&self) -> Result<Vec<HistoryEntry>> {
        let entries = self.read_all()?;
        Ok(entries.into_iter().filter(|e| !e.undone).collect())
    }

    /// Clear all history
    pub fn clear(&self) -> Result<()> {
        if self.path.exists() {
            fs::remove_file(&self.path)?;
        }
        Ok(())
    }

    /// Get history file path
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Create a new history entry
pub fn create_entry(
    id: String,
    original_path: PathBuf,
    new_path: PathBuf,
    ai_suggestion: String,
    category: Option<String>,
    tags: Vec<String>,
    file_hash: String,
) -> HistoryEntry {
    HistoryEntry {
        id,
        timestamp: Utc::now(),
        original_path,
        new_path,
        ai_suggestion,
        category,
        tags,
        file_hash,
        undone: false,
    }
}
