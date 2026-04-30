use std::path::PathBuf;

use crate::cache_cleaner::{CacheDiscovery, CacheSizeState, CleanupPreview, build_cleanup_preview};
use crate::i18n::Language;
use crate::models::{BackgroundTaskStatus, DirectoryEntryInfo};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    CacheCleanup,
    SpaceExplorer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveDialog {
    None,
    Help,
    DeleteConfirmation,
    CleanupSummary,
}

#[derive(Debug, Clone, Default)]
pub struct ExplorerListState {
    entries: Vec<DirectoryEntryInfo>,
    filter: String,
    selected_index: usize,
}

impl ExplorerListState {
    pub fn set_entries(&mut self, mut entries: Vec<DirectoryEntryInfo>) {
        let selected_path = self.selected_entry().map(|entry| entry.path.clone());
        entries.sort_by(|left, right| {
            right
                .size_bytes
                .cmp(&left.size_bytes)
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
        });
        self.entries = entries;
        if let Some(path) = selected_path
            && let Some(index) = self.visible_indices().into_iter().find(|index| {
                self.entries
                    .get(*index)
                    .is_some_and(|entry| entry.path == path)
            })
        {
            self.selected_index = self
                .visible_indices()
                .iter()
                .position(|visible_index| *visible_index == index)
                .unwrap_or(0);
        }
        self.clamp_selection();
    }

    pub fn set_filter(&mut self, filter: String) {
        self.filter = filter;
        self.selected_index = 0;
        self.clamp_selection();
    }

    pub fn visible_entries(&self) -> Vec<&DirectoryEntryInfo> {
        let filter = self.filter.to_lowercase();
        self.entries
            .iter()
            .filter(|item| filter.is_empty() || item.name.to_lowercase().contains(&filter))
            .collect()
    }

    pub fn entries(&self) -> &[DirectoryEntryInfo] {
        &self.entries
    }

    pub fn select_first(&mut self) {
        self.selected_index = 0;
    }

    pub fn select_last(&mut self) {
        let len = self.visible_entries().len();
        self.selected_index = len.saturating_sub(1);
    }

    pub fn select_next(&mut self) {
        let len = self.visible_entries().len();
        if len > 0 {
            self.selected_index = (self.selected_index + 1).min(len - 1);
        }
    }

    pub fn select_previous(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    pub fn page_up(&mut self, step: usize) {
        self.selected_index = self.selected_index.saturating_sub(step.max(1));
    }

    pub fn page_down(&mut self, step: usize) {
        let len = self.visible_entries().len();
        if len > 0 {
            self.selected_index = (self.selected_index + step.max(1)).min(len - 1);
        }
    }

    pub fn selected_entry(&self) -> Option<&DirectoryEntryInfo> {
        let visible_indices = self.visible_indices();
        let index = *visible_indices.get(self.selected_index)?;
        self.entries.get(index)
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn filter(&self) -> &str {
        &self.filter
    }

    fn visible_indices(&self) -> Vec<usize> {
        let filter = self.filter.to_lowercase();
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, item)| filter.is_empty() || item.name.to_lowercase().contains(&filter))
            .map(|(index, _)| index)
            .collect()
    }

    fn clamp_selection(&mut self) {
        let len = self.visible_entries().len();
        if len == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= len {
            self.selected_index = len - 1;
        }
    }
}

#[derive(Debug, Clone)]
pub struct App {
    language: Language,
    page: Page,
    active_dialog: ActiveDialog,
    cache_items: Vec<CacheDiscovery>,
    cache_selected_index: usize,
    explorer_state: ExplorerListState,
    current_path: Option<PathBuf>,
    path_history: Vec<PathBuf>,
    pub filter_input: String,
    pub task_status: Option<BackgroundTaskStatus>,
    pub status_message: String,
    pub last_cleanup_preview: Option<CleanupPreview>,
    pub help_visible: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new(Language::En)
    }
}

impl App {
    pub fn new(language: Language) -> Self {
        Self {
            language,
            page: Page::CacheCleanup,
            active_dialog: ActiveDialog::None,
            cache_items: Vec::new(),
            cache_selected_index: 0,
            explorer_state: ExplorerListState::default(),
            current_path: None,
            path_history: Vec::new(),
            filter_input: String::new(),
            task_status: None,
            status_message: language.help_hint().into(),
            last_cleanup_preview: None,
            help_visible: false,
        }
    }

    pub fn language(&self) -> Language {
        self.language
    }

    pub fn page(&self) -> Page {
        self.page
    }

    pub fn active_dialog(&self) -> ActiveDialog {
        self.active_dialog
    }

    pub fn cache_items(&self) -> &[CacheDiscovery] {
        &self.cache_items
    }

    pub fn cache_items_mut(&mut self) -> &mut [CacheDiscovery] {
        &mut self.cache_items
    }

    pub fn current_path(&self) -> Option<&PathBuf> {
        self.current_path.as_ref()
    }

    pub fn explorer_state(&self) -> &ExplorerListState {
        &self.explorer_state
    }

    pub fn explorer_state_mut(&mut self) -> &mut ExplorerListState {
        &mut self.explorer_state
    }

    pub fn set_cache_items(&mut self, items: Vec<CacheDiscovery>) {
        self.cache_items = items;
        self.cache_selected_index = 0;
    }

    pub fn upsert_cache_item(&mut self, mut item: CacheDiscovery) {
        let selected_kind = self.selected_cache().map(|selected| selected.kind);
        if let Some(index) = self
            .cache_items
            .iter()
            .position(|existing| existing.kind == item.kind)
        {
            item.selected = self.cache_items[index].selected;
            self.cache_items[index] = item;
            return;
        }
        self.cache_items.push(item);
        self.cache_items.sort_by_key(|item| item.label.to_lowercase());
        self.cache_selected_index = self
            .cache_items
            .iter()
            .position(|existing| Some(existing.kind) == selected_kind)
            .unwrap_or(0);
    }

    pub fn set_current_path(&mut self, path: PathBuf) {
        self.path_history = vec![path.clone()];
        self.current_path = Some(path);
    }

    pub fn push_directory(&mut self, path: PathBuf) {
        self.path_history.push(path.clone());
        self.current_path = Some(path);
    }

    pub fn pop_directory(&mut self) {
        if self.path_history.len() > 1 {
            self.path_history.pop();
        }
        self.current_path = self.path_history.last().cloned();
    }

    pub fn path_history(&self) -> &[PathBuf] {
        &self.path_history
    }

    pub fn next_page(&mut self) {
        self.page = match self.page {
            Page::CacheCleanup => Page::SpaceExplorer,
            Page::SpaceExplorer => Page::CacheCleanup,
        };
    }

    pub fn previous_page(&mut self) {
        self.next_page();
    }

    pub fn select_next_cache(&mut self) {
        if !self.cache_items.is_empty() {
            self.cache_selected_index =
                (self.cache_selected_index + 1).min(self.cache_items.len() - 1);
        }
    }

    pub fn select_previous_cache(&mut self) {
        self.cache_selected_index = self.cache_selected_index.saturating_sub(1);
    }

    pub fn toggle_selected_cache(&mut self) {
        if let Some(item) = self.cache_items.get_mut(self.cache_selected_index) {
            item.selected = !item.selected;
        }
    }

    pub fn selected_cache(&self) -> Option<&CacheDiscovery> {
        self.cache_items.get(self.cache_selected_index)
    }

    pub fn selected_cache_index(&self) -> usize {
        self.cache_selected_index
    }

    pub fn toggle_all_caches(&mut self) {
        let should_select = self.cache_items.iter().any(|item| !item.selected);
        for item in &mut self.cache_items {
            item.selected = should_select;
        }
    }

    pub fn open_delete_confirmation(&mut self) {
        let waiting_items = self
            .cache_items
            .iter()
            .filter(|item| item.selected && item.size_state != CacheSizeState::Ready)
            .map(|item| item.label.clone())
            .collect::<Vec<_>>();
        if !waiting_items.is_empty() {
            self.active_dialog = ActiveDialog::None;
            self.status_message = self
                .language
                .wait_for_selected_cache_sizes(&waiting_items.join(", "));
            self.last_cleanup_preview = None;
            return;
        }
        let preview = build_cleanup_preview(&self.cache_items);
        if preview.items.is_empty() {
            self.active_dialog = ActiveDialog::None;
            self.status_message = self.language.select_at_least_one_cache().into();
            self.last_cleanup_preview = None;
            return;
        }
        self.last_cleanup_preview = Some(preview);
        self.active_dialog = ActiveDialog::DeleteConfirmation;
    }

    pub fn close_dialog(&mut self) {
        self.active_dialog = ActiveDialog::None;
        self.help_visible = false;
    }

    pub fn toggle_help(&mut self) {
        self.help_visible = !self.help_visible;
        self.active_dialog = if self.help_visible {
            ActiveDialog::Help
        } else {
            ActiveDialog::None
        };
    }

    pub fn show_cleanup_summary(&mut self) {
        self.active_dialog = ActiveDialog::CleanupSummary;
        self.help_visible = false;
    }
}
