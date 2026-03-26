use std::path::PathBuf;

use sysclean::cache_cleaner::{CacheDiscovery, CacheTargetKind, build_cleanup_preview};

#[test]
fn cleanup_preview_sums_only_selected_items() {
    let mut npm = CacheDiscovery::new(
        CacheTargetKind::Npm,
        "npm".into(),
        vec![PathBuf::from("npm")],
    );
    npm.selected = true;
    npm.reclaimable_bytes = Some(500);

    let mut cargo = CacheDiscovery::new(
        CacheTargetKind::Cargo,
        "cargo".into(),
        vec![PathBuf::from("cargo")],
    );
    cargo.selected = false;
    cargo.reclaimable_bytes = Some(800);

    let preview = build_cleanup_preview(&[npm, cargo]);

    assert_eq!(preview.items.len(), 1);
    assert_eq!(preview.total_reclaimable_bytes, 500);
}
