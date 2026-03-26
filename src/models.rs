use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScanState {
    Pending,
    Scanning,
    Ready,
    Cached,
    Skipped,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectoryEntryInfo {
    pub name: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub can_enter: bool,
    pub scan_state: ScanState,
    pub message: Option<String>,
}

impl DirectoryEntryInfo {
    pub fn new_pending(name: String, path: PathBuf, can_enter: bool) -> Self {
        Self {
            name,
            path,
            size_bytes: 0,
            can_enter,
            scan_state: ScanState::Pending,
            message: None,
        }
    }

    pub fn mark_scanning(&mut self) {
        self.scan_state = ScanState::Scanning;
    }

    pub fn new_ready(name: String, path: PathBuf, size_bytes: u64, can_enter: bool) -> Self {
        Self {
            name,
            path,
            size_bytes,
            can_enter,
            scan_state: ScanState::Ready,
            message: None,
        }
    }

    pub fn new_cached(name: String, path: PathBuf, size_bytes: u64, can_enter: bool) -> Self {
        Self {
            name,
            path,
            size_bytes,
            can_enter,
            scan_state: ScanState::Cached,
            message: None,
        }
    }

    pub fn new_skipped(name: String, path: PathBuf, message: impl Into<String>) -> Self {
        Self {
            name,
            path,
            size_bytes: 0,
            can_enter: false,
            scan_state: ScanState::Skipped,
            message: Some(message.into()),
        }
    }

    pub fn new_error(name: String, path: PathBuf, message: impl Into<String>) -> Self {
        Self {
            name,
            path,
            size_bytes: 0,
            can_enter: false,
            scan_state: ScanState::Error,
            message: Some(message.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackgroundTaskStatus {
    pub title: String,
    pub message: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub progress_current: usize,
    pub progress_total: usize,
    pub cancellable: bool,
}

impl BackgroundTaskStatus {
    pub fn new(title: impl Into<String>, message: impl Into<String>, cancellable: bool) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            started_at: Utc::now(),
            completed_at: None,
            progress_current: 0,
            progress_total: 0,
            cancellable,
        }
    }
}
