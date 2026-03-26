use std::path::PathBuf;

use sysclean::app::{ActiveDialog, App, Page};
use sysclean::cache_cleaner::{CacheDiscovery, CacheTargetKind};

#[test]
fn app_switches_between_workspace_pages() {
    let mut app = App::default();

    assert_eq!(app.page(), Page::CacheCleanup);
    app.next_page();
    assert_eq!(app.page(), Page::SpaceExplorer);
    app.next_page();
    assert_eq!(app.page(), Page::CacheCleanup);
}

#[test]
fn app_toggle_cache_selection_marks_current_item() {
    let mut app = App::default();
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
    let mut app = App::default();
    app.set_cache_items(vec![CacheDiscovery::new(
        CacheTargetKind::Npm,
        "npm".into(),
        vec![PathBuf::from("npm")],
    )]);

    app.open_delete_confirmation();
    assert_eq!(app.active_dialog(), ActiveDialog::None);

    app.toggle_selected_cache();
    app.open_delete_confirmation();
    assert_eq!(app.active_dialog(), ActiveDialog::DeleteConfirmation);
}

#[test]
fn app_directory_navigation_tracks_current_path() {
    let mut app = App::default();
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
