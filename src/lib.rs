// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

//! Panoptes: Local AI File Scanner & Renamer
//!
//! A comprehensive file analysis and organization system using local AI models.
//! Version 3.0 - Full plugin architecture with web UI and database support.

pub mod analyzers;
pub mod config;
pub mod db;
pub mod error;
pub mod history;
pub mod ollama;
pub mod watcher;
pub mod web;

pub use config::AppConfig;
pub use error::{PanoptesError, Result};
