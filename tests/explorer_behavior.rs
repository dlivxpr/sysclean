use std::path::PathBuf;

use sysclean::app::ExplorerListState;
use sysclean::models::DirectoryEntryInfo;

#[test]
fn explorer_state_sorts_entries_by_size_descending() {
    let mut state = ExplorerListState::default();
    state.set_entries(vec![
        DirectoryEntryInfo::new_ready("small".into(), PathBuf::from("small"), 10, true),
        DirectoryEntryInfo::new_ready("large".into(), PathBuf::from("large"), 100, true),
        DirectoryEntryInfo::new_ready("medium".into(), PathBuf::from("medium"), 50, true),
    ]);

    let visible: Vec<String> = state
        .visible_entries()
        .iter()
        .map(|item| item.name.clone())
        .collect();
    assert_eq!(visible, vec!["large", "medium", "small"]);
}

#[test]
fn explorer_state_filters_by_name_without_losing_original_ordering() {
    let mut state = ExplorerListState::default();
    state.set_entries(vec![
        DirectoryEntryInfo::new_ready(
            "node_modules".into(),
            PathBuf::from("node_modules"),
            500,
            true,
        ),
        DirectoryEntryInfo::new_ready("notes".into(), PathBuf::from("notes"), 100, true),
        DirectoryEntryInfo::new_ready("npm-cache".into(), PathBuf::from("npm-cache"), 300, true),
    ]);

    state.set_filter("np".into());

    let visible: Vec<String> = state
        .visible_entries()
        .iter()
        .map(|item| item.name.clone())
        .collect();
    assert_eq!(visible, vec!["npm-cache"]);
}

#[test]
fn explorer_state_home_and_end_jump_to_extremes() {
    let mut state = ExplorerListState::default();
    state.set_entries(vec![
        DirectoryEntryInfo::new_ready("a".into(), PathBuf::from("a"), 10, true),
        DirectoryEntryInfo::new_ready("b".into(), PathBuf::from("b"), 20, true),
        DirectoryEntryInfo::new_ready("c".into(), PathBuf::from("c"), 30, true),
    ]);

    state.select_last();
    assert_eq!(state.selected_entry().expect("selected").name, "a");

    state.select_first();
    assert_eq!(state.selected_entry().expect("selected").name, "c");
}

#[test]
fn explorer_state_keeps_selection_on_same_path_after_resort() {
    let mut state = ExplorerListState::default();
    let alpha_path = PathBuf::from("alpha");
    let beta_path = PathBuf::from("beta");

    state.set_entries(vec![
        DirectoryEntryInfo {
            name: "alpha".into(),
            path: alpha_path.clone(),
            size_bytes: 10,
            can_enter: true,
            scan_state: sysclean::models::ScanState::Ready,
            message: None,
        },
        DirectoryEntryInfo {
            name: "beta".into(),
            path: beta_path.clone(),
            size_bytes: 5,
            can_enter: true,
            scan_state: sysclean::models::ScanState::Ready,
            message: None,
        },
    ]);

    state.select_next();
    assert_eq!(state.selected_entry().expect("selected").path, beta_path);

    state.set_entries(vec![
        DirectoryEntryInfo {
            name: "alpha".into(),
            path: alpha_path,
            size_bytes: 10,
            can_enter: true,
            scan_state: sysclean::models::ScanState::Ready,
            message: None,
        },
        DirectoryEntryInfo {
            name: "beta".into(),
            path: beta_path.clone(),
            size_bytes: 100,
            can_enter: true,
            scan_state: sysclean::models::ScanState::Ready,
            message: None,
        },
    ]);

    assert_eq!(state.selected_entry().expect("selected").path, beta_path);
}
