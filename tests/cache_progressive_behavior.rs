use std::path::PathBuf;
use tempfile::tempdir;

use sysclean::cache_cleaner::{
    CacheDiscovery, CacheSizeState, CacheTargetKind, build_cleanup_preview,
    execute_cleanup_with_progress, populate_cache_size,
};

#[test]
fn cache_discovery_can_exist_before_size_calculation_finishes() {
    let item = CacheDiscovery::new(
        CacheTargetKind::Uv,
        "uv".into(),
        vec![PathBuf::from(r"C:\cache\uv")],
    );

    assert_eq!(item.size_state, CacheSizeState::Pending);
    assert_eq!(item.reclaimable_bytes, None);
    assert_eq!(item.total_bytes, 0);
}

#[test]
fn cleanup_preview_excludes_selected_items_that_are_not_ready() {
    let mut ready = CacheDiscovery::new(
        CacheTargetKind::Cargo,
        "cargo".into(),
        vec![PathBuf::from(r"C:\cache\cargo")],
    );
    ready.selected = true;
    ready.size_state = CacheSizeState::Ready;
    ready.reclaimable_bytes = Some(256);

    let mut scanning = CacheDiscovery::new(
        CacheTargetKind::Npm,
        "npm".into(),
        vec![PathBuf::from(r"C:\cache\npm")],
    );
    scanning.selected = true;
    scanning.size_state = CacheSizeState::Scanning;

    let preview = build_cleanup_preview(&[ready, scanning]);

    assert_eq!(preview.items.len(), 1);
    assert_eq!(preview.items[0].label, "cargo");
    assert_eq!(preview.total_reclaimable_bytes, 256);
}

#[test]
fn populate_cache_size_records_estimates_for_each_path() {
    let temp = tempdir().expect("temp dir");
    let first = temp.path().join("one");
    let second = temp.path().join("two");
    std::fs::create_dir_all(&first).expect("first dir");
    std::fs::create_dir_all(&second).expect("second dir");
    std::fs::write(first.join("a.bin"), vec![1_u8; 16]).expect("first file");
    std::fs::write(second.join("b.bin"), vec![1_u8; 32]).expect("second file");

    let mut item = CacheDiscovery::new(
        CacheTargetKind::Cargo,
        "cargo".into(),
        vec![first.clone(), second.clone()],
    );

    populate_cache_size(&mut item).expect("size calculation");

    assert_eq!(item.total_bytes, 48);
    assert_eq!(item.path_estimates.len(), 2);
    assert_eq!(item.path_estimates[0].path, first);
    assert_eq!(item.path_estimates[0].estimated_bytes, Some(16));
    assert_eq!(item.path_estimates[1].path, second);
    assert_eq!(item.path_estimates[1].estimated_bytes, Some(32));
}

#[test]
fn cleanup_progress_reports_estimated_bytes_per_path() {
    let temp = tempdir().expect("temp dir");
    let first = temp.path().join("cargo-registry");
    let second = temp.path().join("cargo-git");
    std::fs::create_dir_all(&first).expect("first dir");
    std::fs::create_dir_all(&second).expect("second dir");
    std::fs::write(first.join("a.bin"), vec![1_u8; 10]).expect("first file");
    std::fs::write(second.join("b.bin"), vec![1_u8; 20]).expect("second file");

    let mut cargo = CacheDiscovery::new(
        CacheTargetKind::Cargo,
        "cargo".into(),
        vec![first.clone(), second.clone()],
    );
    cargo.selected = true;
    cargo.size_state = CacheSizeState::Ready;
    cargo.reclaimable_bytes = Some(30);
    cargo.total_bytes = 30;
    cargo.path_estimates[0].estimated_bytes = Some(10);
    cargo.path_estimates[1].estimated_bytes = Some(20);

    let mut completed_bytes = Vec::new();
    let results = execute_cleanup_with_progress(
        &[cargo],
        &sysclean::cache_cleaner::SystemCommandRunner,
        |progress| {
            completed_bytes.push(progress.completed_bytes);
        },
    );

    assert_eq!(results.len(), 1);
    assert_eq!(completed_bytes, vec![10, 30]);
}
