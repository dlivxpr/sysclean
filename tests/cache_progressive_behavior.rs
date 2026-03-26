use std::path::PathBuf;

use sysclean::cache_cleaner::{
    CacheDiscovery, CacheSizeState, CacheTargetKind, build_cleanup_preview,
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
