// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

//! File system watcher for monitoring directories

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::Result;

/// Events emitted by the watcher
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// A new file was created
    FileCreated(PathBuf),
    /// A file was modified
    FileModified(PathBuf),
    /// A file was deleted
    FileDeleted(PathBuf),
    /// A file was renamed
    FileRenamed { from: PathBuf, to: PathBuf },
    /// Watcher error
    Error(String),
}

/// File system watcher
pub struct FileWatcher {
    watcher: RecommendedWatcher,
    watched_paths: Vec<PathBuf>,
    event_rx: Receiver<notify::Result<Event>>,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new() -> Result<Self> {
        let (tx, rx) = channel();

        let config = Config::default()
            .with_poll_interval(Duration::from_secs(2));

        let watcher = RecommendedWatcher::new(tx, config)?;

        Ok(Self {
            watcher,
            watched_paths: Vec::new(),
            event_rx: rx,
        })
    }

    /// Add a directory to watch
    pub fn watch(&mut self, path: &Path) -> Result<()> {
        // Create directory if it doesn't exist
        if !path.exists() {
            std::fs::create_dir_all(path)?;
            info!("Created watch directory: {:?}", path);
        }

        self.watcher.watch(path, RecursiveMode::NonRecursive)?;
        self.watched_paths.push(path.to_path_buf());
        info!("Watching: {:?}", path);

        Ok(())
    }

    /// Stop watching a directory
    pub fn unwatch(&mut self, path: &Path) -> Result<()> {
        self.watcher.unwatch(path)?;
        self.watched_paths.retain(|p| p != path);
        info!("Stopped watching: {:?}", path);
        Ok(())
    }

    /// Get the next event (blocking with timeout)
    pub fn next_event(&self, timeout: Duration) -> Option<WatchEvent> {
        match self.event_rx.recv_timeout(timeout) {
            Ok(Ok(event)) => Self::convert_event(event),
            Ok(Err(e)) => Some(WatchEvent::Error(e.to_string())),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                Some(WatchEvent::Error("Watcher disconnected".to_string()))
            }
        }
    }

    /// Convert notify event to our event type
    fn convert_event(event: Event) -> Option<WatchEvent> {
        match event.kind {
            EventKind::Create(_) => {
                event.paths.first().map(|p| WatchEvent::FileCreated(p.clone()))
            }
            EventKind::Modify(_) => {
                event.paths.first().map(|p| WatchEvent::FileModified(p.clone()))
            }
            EventKind::Remove(_) => {
                event.paths.first().map(|p| WatchEvent::FileDeleted(p.clone()))
            }
            _ => None,
        }
    }

    /// Get currently watched paths
    pub fn watched_paths(&self) -> &[PathBuf] {
        &self.watched_paths
    }
}

/// Check if a file should be processed
pub fn should_process(path: &Path) -> bool {
    let filename = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return false,
    };

    // Skip hidden files
    if filename.starts_with('.') {
        return false;
    }

    // Skip temporary files
    let temp_extensions = [".tmp", ".part", ".crdownload", ".partial", ".download"];
    for ext in &temp_extensions {
        if filename.ends_with(ext) {
            return false;
        }
    }

    // Skip system files
    let skip_names = ["desktop.ini", "thumbs.db", ".ds_store"];
    if skip_names.iter().any(|n| filename.eq_ignore_ascii_case(n)) {
        return false;
    }

    true
}

/// Wait for file to be stable (not being written)
pub async fn wait_for_stable(path: &Path, max_wait: Duration) -> bool {
    let check_interval = Duration::from_millis(500);
    let start = std::time::Instant::now();

    let mut last_size = match std::fs::metadata(path) {
        Ok(m) => m.len(),
        Err(_) => return false,
    };

    loop {
        tokio::time::sleep(check_interval).await;

        // Check if we've exceeded max wait time
        if start.elapsed() > max_wait {
            warn!("File stability check timed out for {:?}", path);
            return true; // Proceed anyway
        }

        // Check if file still exists
        let current_size = match std::fs::metadata(path) {
            Ok(m) => m.len(),
            Err(_) => return false, // File was deleted
        };

        // If size hasn't changed, file is stable
        if current_size == last_size {
            return true;
        }

        last_size = current_size;
        debug!("File {:?} still being written, size: {}", path, current_size);
    }
}
