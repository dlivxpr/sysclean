use std::path::PathBuf;

use sysclean::app::{ActiveDialog, App, Page};
use sysclean::cache_cleaner::{CacheDiscovery, CacheSizeState, CacheTargetKind};
use sysclean::i18n::Language;
use sysclean::models::{DirectoryEntryInfo, ScanState};

#[test]
fn app_switches_between_workspace_pages() {
    let mut app = App::new(Language::En);

    assert_eq!(app.page(), Page::CacheCleanup);
    app.next_page();
    assert_eq!(app.page(), Page::SpaceExplorer);
    app.next_page();
    assert_eq!(app.page(), Page::CacheCleanup);
}

#[test]
fn app_toggle_cache_selection_marks_current_item() {
    let mut app = App::new(Language::En);
    app.set_cache_items(vec![
        CacheDiscovery::new(
            CacheTargetKind::Npm,
            "npm".into(),
            vec![PathBuf::from("npm")],
        ),
        CacheDiscovery::new(
            CacheTargetKind::Cargo,
            "cargo".into(),
            vec![PathBuf::from("cargo")],
        ),
    ]);

    assert!(!app.cache_items()[0].selected);
    app.toggle_selected_cache();
    assert!(app.cache_items()[0].selected);
}

#[test]
fn app_delete_requires_selected_cache_items() {
    let mut app = App::new(Language::En);
    let mut cache = CacheDiscovery::new(
        CacheTargetKind::Npm,
        "npm".into(),
        vec![PathBuf::from("npm")],
    );
    cache.size_state = CacheSizeState::Ready;
    cache.reclaimable_bytes = Some(128);
    app.set_cache_items(vec![cache]);

    app.open_delete_confirmation();
    assert_eq!(app.active_dialog(), ActiveDialog::None);

    app.toggle_selected_cache();
    app.open_delete_confirmation();
    assert_eq!(app.active_dialog(), ActiveDialog::DeleteConfirmation);
}

#[test]
fn app_delete_confirmation_waits_for_selected_cache_size_to_finish() {
    let mut app = App::new(Language::En);
    let mut npm = CacheDiscovery::new(
        CacheTargetKind::Npm,
        "npm".into(),
        vec![PathBuf::from("npm")],
    );
    npm.size_state = CacheSizeState::Scanning;
    app.set_cache_items(vec![npm]);

    app.toggle_selected_cache();
    app.open_delete_confirmation();

    assert_eq!(app.active_dialog(), ActiveDialog::None);
    assert!(app.status_message.contains("finish"));
}

#[test]
fn app_directory_navigation_tracks_current_path() {
    let mut app = App::new(Language::En);
    app.set_current_path(PathBuf::from(r"C:\Users\demo"));
    app.push_directory(PathBuf::from(r"C:\Users\demo\Downloads"));

    assert_eq!(
        app.current_path().expect("current path"),
        &PathBuf::from(r"C:\Users\demo\Downloads")
    );

    app.pop_directory();
    assert_eq!(
        app.current_path().expect("current path"),
        &PathBuf::from(r"C:\Users\demo")
    );
}

#[test]
fn app_directory_navigation_can_show_skeleton_entries_before_sizes_are_ready() {
    let mut app = App::new(Language::En);
    app.set_current_path(PathBuf::from(r"C:\Users\demo"));
    app.push_directory(PathBuf::from(r"C:\Users\demo\Projects"));
    app.explorer_state_mut().set_entries(vec![
        DirectoryEntryInfo {
            name: "alpha".into(),
            path: PathBuf::from(r"C:\Users\demo\Projects\alpha"),
            size_bytes: 0,
            can_enter: true,
            scan_state: ScanState::Pending,
            message: None,
        },
        DirectoryEntryInfo {
            name: "beta".into(),
            path: PathBuf::from(r"C:\Users\demo\Projects\beta"),
            size_bytes: 0,
            can_enter: true,
            scan_state: ScanState::Scanning,
            message: None,
        },
    ]);

    let visible: Vec<(String, ScanState)> = app
        .explorer_state()
        .visible_entries()
        .iter()
        .map(|item| (item.name.clone(), item.scan_state))
        .collect();

    assert_eq!(
        visible,
        vec![
            ("alpha".to_string(), ScanState::Pending),
            ("beta".to_string(), ScanState::Scanning),
        ]
    );
}

#[test]
fn app_cache_upsert_preserves_selection_and_checked_state() {
    let mut app = App::new(Language::En);
    let mut npm = CacheDiscovery::new(
        CacheTargetKind::Npm,
        "npm".into(),
        vec![PathBuf::from("npm")],
    );
    npm.size_state = CacheSizeState::Pending;
    app.set_cache_items(vec![npm]);

    app.toggle_selected_cache();

    let mut updated = CacheDiscovery::new(
        CacheTargetKind::Npm,
        "npm".into(),
        vec![PathBuf::from("npm")],
    );
    updated.size_state = CacheSizeState::Ready;
    updated.reclaimable_bytes = Some(256);
    updated.total_bytes = 256;
    app.upsert_cache_item(updated);

    assert_eq!(app.selected_cache().expect("selected").label, "npm");
    assert!(app.selected_cache().expect("selected").selected);
    assert_eq!(
        app.selected_cache().expect("selected").size_state,
        CacheSizeState::Ready
    );
}

#[test]
fn app_default_status_message_is_localized() {
    let english = App::new(Language::En);
    let chinese = App::new(Language::ZhCn);

    assert_eq!(english.status_message, "Press ? for help");
    assert_eq!(chinese.status_message, "按 ? 查看帮助");
}
