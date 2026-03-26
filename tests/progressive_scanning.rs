use sysclean::cache_cleaner::{
    CacheDiscovery, CacheSizeState, CacheTargetKind, populate_cache_size,
};
use sysclean::models::ScanState;
use sysclean::space_explorer::discover_directory_skeleton;
use tempfile::tempdir;

#[test]
fn directory_skeleton_discovers_children_before_sizes_are_computed() {
    let temp = tempdir().expect("temp dir");
    std::fs::create_dir_all(temp.path().join("alpha")).expect("alpha");
    std::fs::create_dir_all(temp.path().join("beta")).expect("beta");

    let entries = discover_directory_skeleton(temp.path()).expect("skeleton");

    assert_eq!(entries.len(), 2);
    assert!(entries.iter().all(|entry| entry.size_bytes == 0));
    assert!(
        entries
            .iter()
            .all(|entry| entry.scan_state == ScanState::Pending)
    );
}

#[test]
fn populate_cache_size_marks_entry_ready_after_size_calculation() {
    let temp = tempdir().expect("temp dir");
    let cache_path = temp.path().join("uv-cache");
    std::fs::create_dir_all(&cache_path).expect("cache dir");
    std::fs::write(cache_path.join("wheel.whl"), vec![1_u8; 32]).expect("cache file");

    let mut item = CacheDiscovery::new(CacheTargetKind::Uv, "uv".into(), vec![cache_path]);
    item.size_state = CacheSizeState::Scanning;

    populate_cache_size(&mut item).expect("size calculation");

    assert_eq!(item.size_state, CacheSizeState::Ready);
    assert_eq!(item.total_bytes, 32);
    assert_eq!(item.reclaimable_bytes, Some(32));
}
