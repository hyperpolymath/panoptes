// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

//! Panoptes Undo Utility
//!
//! Reverses file renames recorded in the history log.

use clap::Parser;
use serde::Deserialize;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "panoptes-undo")]
#[command(version = "1.0.0")]
#[command(about = "Undo Panoptes file renames")]
struct Args {
    /// Path to history file
    #[arg(short, long, default_value = "panoptes_history.jsonl")]
    history_file: PathBuf,

    /// Number of renames to undo (default: 1, use 0 for all)
    #[arg(short, long, default_value = "1")]
    count: usize,

    /// Dry run - show what would be undone without doing it
    #[arg(long)]
    dry_run: bool,

    /// List all entries in history
    #[arg(long)]
    list: bool,
}

#[derive(Deserialize, Debug)]
struct HistoryEntry {
    timestamp: String,
    original_path: String,
    new_path: String,
    ai_suggestion: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if !args.history_file.exists() {
        eprintln!("History file not found: {:?}", args.history_file);
        eprintln!("No renames to undo.");
        return Ok(());
    }

    let file = File::open(&args.history_file)?;
    let reader = BufReader::new(file);

    let mut entries: Vec<HistoryEntry> = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str(&line) {
            Ok(entry) => entries.push(entry),
            Err(e) => eprintln!("Warning: Failed to parse history entry: {}", e),
        }
    }

    if entries.is_empty() {
        println!("No history entries found.");
        return Ok(());
    }

    if args.list {
        println!("Rename History ({} entries):", entries.len());
        println!("{:-<80}", "");
        for (i, entry) in entries.iter().rev().enumerate() {
            println!(
                "{:3}. [{}] {} -> {}",
                i + 1,
                &entry.timestamp[..19], // Trim timezone
                entry.original_path,
                entry.new_path
            );
            println!("     AI suggestion: {}", entry.ai_suggestion);
        }
        return Ok(());
    }

    // Reverse entries to undo most recent first
    entries.reverse();

    let count = if args.count == 0 {
        entries.len()
    } else {
        args.count.min(entries.len())
    };

    println!(
        "{}Undoing {} rename(s)...",
        if args.dry_run { "[DRY RUN] " } else { "" },
        count
    );

    let mut undone = 0;
    let mut failed = 0;

    for entry in entries.iter().take(count) {
        let new_path = PathBuf::from(&entry.new_path);
        let original_path = PathBuf::from(&entry.original_path);

        if !new_path.exists() {
            eprintln!(
                "  Skip: {} (file not found, may have been moved/deleted)",
                entry.new_path
            );
            failed += 1;
            continue;
        }

        if original_path.exists() {
            eprintln!(
                "  Skip: {} (original path already exists)",
                entry.original_path
            );
            failed += 1;
            continue;
        }

        if args.dry_run {
            println!("  Would rename: {} -> {}", entry.new_path, entry.original_path);
        } else {
            match fs::rename(&new_path, &original_path) {
                Ok(()) => {
                    println!("  Undone: {} -> {}", entry.new_path, entry.original_path);
                    undone += 1;
                }
                Err(e) => {
                    eprintln!("  Failed: {} ({})", entry.new_path, e);
                    failed += 1;
                }
            }
        }
    }

    println!();
    if args.dry_run {
        println!("Dry run complete. {} rename(s) would be undone.", count - failed);
    } else {
        println!(
            "Done. {} undone, {} failed/skipped.",
            undone, failed
        );
        if undone > 0 {
            println!("Note: History file not modified. Run again to undo more.");
        }
    }

    Ok(())
}
