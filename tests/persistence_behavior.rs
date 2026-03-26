use std::path::PathBuf;

use chrono::{Duration, Utc};
use sysclean::models::{DirectoryEntryInfo, ScanState};
use sysclean::persistence::{CacheSnapshot, ScanCache};
use sysclean::space_explorer::load_directory_entries;
use tempfile::tempdir;

#[test]
fn cache_snapshot_is_fresh_within_ttl() {
    let snapshot = CacheSnapshot {
        path: PathBuf::from(r"C:\Users\demo"),
        captured_at: Utc::now() - Duration::hours(2),
        entries: vec![],
    };

    assert!(snapshot.is_fresh(Duration::hours(24)));
    assert!(!snapshot.is_fresh(Duration::minutes(30)));
}

#[test]
fn scan_cache_round_trips_snapshots_to_disk() {
    let temp = tempdir().expect("temp dir");
    let cache = ScanCache::new(temp.path().join("scan-cache.json"));
    let snapshot = CacheSnapshot {
        path: PathBuf::from(r"C:\Users\demo\Downloads"),
        captured_at: Utc::now(),
        entries: vec![DirectoryEntryInfo::new_ready(
            "SDKs".into(),
            PathBuf::from(r"C:\Users\demo\Downloads\SDKs"),
            512,
            true,
        )],
    };

    cache.save_snapshot(&snapshot).expect("save snapshot");
    let loaded = cache
        .load_snapshot(&snapshot.path)
        .expect("load snapshot")
        .expect("snapshot should exist");

    assert_eq!(loaded.path, snapshot.path);
    assert_eq!(loaded.entries[0].name, "SDKs");
    assert_eq!(loaded.entries[0].scan_state, ScanState::Ready);
}

#[test]
fn cached_directory_entries_are_marked_as_cached_when_reused() {
    let temp = tempdir().expect("temp dir");
    let cache = ScanCache::new(temp.path().join("scan-cache.json"));
    let path = temp.path().join("workspace");
    std::fs::create_dir_all(&path).expect("workspace");
    let snapshot = CacheSnapshot {
        path: path.clone(),
        captured_at: Utc::now(),
        entries: vec![DirectoryEntryInfo::new_ready(
            "artifacts".into(),
            path.join("artifacts"),
            1024,
            true,
        )],
    };

    cache.save_snapshot(&snapshot).expect("save snapshot");
    let (entries, from_cache) = load_directory_entries(&path, &cache).expect("load entries");

    assert!(from_cache);
    assert_eq!(entries[0].scan_state, ScanState::Cached);
}
