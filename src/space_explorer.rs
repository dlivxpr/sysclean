use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{Duration, Utc};

use crate::cache_cleaner::compute_path_size;
use crate::models::DirectoryEntryInfo;
use crate::persistence::{CacheSnapshot, ScanCache};

pub fn load_directory_entries(
    path: &Path,
    cache: &ScanCache,
) -> Result<(Vec<DirectoryEntryInfo>, bool)> {
    if let Some(snapshot) = cache.load_snapshot(path)?
        && snapshot.is_fresh(Duration::hours(24))
    {
        let entries = snapshot
            .entries
            .into_iter()
            .map(|mut entry| {
                if matches!(entry.scan_state, crate::models::ScanState::Ready) {
                    entry.scan_state = crate::models::ScanState::Cached;
                }
                entry
            })
            .collect();
        return Ok((entries, true));
    }

    let entries = scan_directory_entries(path)?;
    let snapshot = CacheSnapshot {
        path: path.to_path_buf(),
        captured_at: Utc::now(),
        entries: entries.clone(),
    };
    cache.save_snapshot(&snapshot)?;
    Ok((entries, false))
}

pub fn scan_directory_entries(path: &Path) -> Result<Vec<DirectoryEntryInfo>> {
    let mut items = Vec::new();
    for entry in fs::read_dir(path).with_context(|| format!("failed to read {}", path.display()))? {
        let entry = match entry {
            Ok(value) => value,
            Err(error) => {
                items.push(DirectoryEntryInfo::new_error(
                    "<未知>".into(),
                    PathBuf::from(path),
                    error.to_string(),
                ));
                continue;
            }
        };
        let child_path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let metadata = match fs::symlink_metadata(&child_path) {
            Ok(metadata) => metadata,
            Err(error) => {
                items.push(DirectoryEntryInfo::new_error(
                    name,
                    child_path,
                    error.to_string(),
                ));
                continue;
            }
        };

        if metadata.file_type().is_symlink() {
            items.push(DirectoryEntryInfo::new_skipped(
                name,
                child_path,
                "符号链接或 junction 已跳过",
            ));
            continue;
        }
        if !metadata.is_dir() {
            continue;
        }
        match compute_path_size(&child_path) {
            Ok(size) => items.push(DirectoryEntryInfo::new_ready(name, child_path, size, true)),
            Err(error) => items.push(DirectoryEntryInfo::new_error(
                name,
                child_path,
                error.to_string(),
            )),
        }
    }
    items.sort_by(|left, right| {
        right
            .size_bytes
            .cmp(&left.size_bytes)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    Ok(items)
}
