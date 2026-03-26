use std::path::PathBuf;

use sysclean::app::ExplorerListState;
use sysclean::models::DirectoryEntryInfo;

#[test]
fn explorer_state_page_down_moves_selection_forward() {
    let mut state = ExplorerListState::default();
    state.set_entries(vec![
        DirectoryEntryInfo::new_ready("a".into(), PathBuf::from("a"), 500, true),
        DirectoryEntryInfo::new_ready("b".into(), PathBuf::from("b"), 400, true),
        DirectoryEntryInfo::new_ready("c".into(), PathBuf::from("c"), 300, true),
        DirectoryEntryInfo::new_ready("d".into(), PathBuf::from("d"), 200, true),
    ]);

    state.page_down(2);

    assert_eq!(state.selected_entry().expect("selected").name, "c");
}

#[test]
fn explorer_state_page_up_moves_selection_backward() {
    let mut state = ExplorerListState::default();
    state.set_entries(vec![
        DirectoryEntryInfo::new_ready("a".into(), PathBuf::from("a"), 500, true),
        DirectoryEntryInfo::new_ready("b".into(), PathBuf::from("b"), 400, true),
        DirectoryEntryInfo::new_ready("c".into(), PathBuf::from("c"), 300, true),
        DirectoryEntryInfo::new_ready("d".into(), PathBuf::from("d"), 200, true),
    ]);

    state.page_down(3);
    state.page_up(2);

    assert_eq!(state.selected_entry().expect("selected").name, "b");
}
