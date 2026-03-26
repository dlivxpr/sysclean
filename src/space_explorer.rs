use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{Duration as ChronoDuration, Utc};

use crate::cache_cleaner::compute_path_size;
use crate::models::DirectoryEntryInfo;
use crate::persistence::{CacheSnapshot, ScanCache};

const MAX_SCAN_WORKERS: usize = 6;
const MIN_SCAN_WORKERS: usize = 2;
pub const DIRECTORY_UPDATE_BATCH_SIZE: usize = 4;
pub const DIRECTORY_UPDATE_THROTTLE: Duration = Duration::from_millis(125);

pub fn recommended_worker_count(job_count: usize) -> usize {
    if job_count <= 1 {
        return 1;
    }
    let available = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(MIN_SCAN_WORKERS);
    available
        .clamp(MIN_SCAN_WORKERS, MAX_SCAN_WORKERS)
        .min(job_count)
}

pub fn discover_directory_skeleton(path: &Path) -> Result<Vec<DirectoryEntryInfo>> {
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
        items.push(DirectoryEntryInfo::new_pending(name, child_path, true));
    }
    items.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    Ok(items)
}

pub fn load_directory_entries(
    path: &Path,
    cache: &ScanCache,
) -> Result<(Vec<DirectoryEntryInfo>, bool)> {
    if let Some(snapshot) = cache.load_snapshot(path)?
        && snapshot.is_fresh(ChronoDuration::hours(24))
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
    let mut items = discover_directory_skeleton(path)?;
    for item in &mut items {
        if !item.can_enter || item.scan_state == crate::models::ScanState::Skipped {
            continue;
        }
        item.mark_scanning();
        match compute_path_size(&item.path) {
            Ok(size) => {
                item.size_bytes = size;
                item.scan_state = crate::models::ScanState::Ready;
            }
            Err(error) => {
                item.size_bytes = 0;
                item.scan_state = crate::models::ScanState::Error;
                item.message = Some(error.to_string());
            }
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
