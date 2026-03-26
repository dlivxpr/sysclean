use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::models::DirectoryEntryInfo;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheSnapshot {
    pub path: PathBuf,
    pub captured_at: DateTime<Utc>,
    pub entries: Vec<DirectoryEntryInfo>,
}

impl CacheSnapshot {
    pub fn is_fresh(&self, ttl: Duration) -> bool {
        Utc::now() - self.captured_at <= ttl
    }
}

#[derive(Debug, Clone)]
pub struct ScanCache {
    file_path: PathBuf,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ScanCacheFile {
    snapshots: Vec<CacheSnapshot>,
}

impl ScanCache {
    pub fn new(file_path: PathBuf) -> Self {
        Self { file_path }
    }

    pub fn save_snapshot(&self, snapshot: &CacheSnapshot) -> Result<()> {
        let mut file = self.read_cache_file().unwrap_or_default();
        file.snapshots.retain(|item| item.path != snapshot.path);
        file.snapshots.push(snapshot.clone());
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create cache directory {}", parent.display())
            })?;
        }
        let payload = serde_json::to_string_pretty(&file)?;
        fs::write(&self.file_path, payload)
            .with_context(|| format!("failed to write cache file {}", self.file_path.display()))?;
        Ok(())
    }

    pub fn load_snapshot(&self, path: &Path) -> Result<Option<CacheSnapshot>> {
        let file = self.read_cache_file().unwrap_or_default();
        Ok(file.snapshots.into_iter().find(|item| item.path == path))
    }

    fn read_cache_file(&self) -> Result<ScanCacheFile> {
        if !self.file_path.exists() {
            return Ok(ScanCacheFile::default());
        }
        let payload = fs::read_to_string(&self.file_path)
            .with_context(|| format!("failed to read cache file {}", self.file_path.display()))?;
        Ok(serde_json::from_str(&payload)?)
    }
}
