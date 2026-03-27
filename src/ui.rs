use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::border;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Cell, Clear, LineGauge, Paragraph, Row, Table, TableState, Tabs, Wrap,
};

use crate::app::{ActiveDialog, App, Page};
use crate::cache_cleaner::{CacheDiscovery, CacheSizeState, CleanupOutcome};
use crate::i18n::Language;
use crate::models::{BackgroundTaskStatus, DirectoryEntryInfo, ScanState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Filtering,
}

pub fn render(
    frame: &mut Frame,
    app: &App,
    input_mode: InputMode,
    cleanup_results: &[CleanupOutcome],
) {
    let area = frame.area();
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);

    render_header(frame, vertical[0], app);
    match app.page() {
        Page::CacheCleanup => render_cache_page(frame, vertical[1], app),
        Page::SpaceExplorer => render_explorer_page(frame, vertical[1], app, input_mode),
    }
    render_footer(frame, vertical[2], app, input_mode);

    match app.active_dialog() {
        ActiveDialog::Help => render_help_dialog(frame, area, app.language()),
        ActiveDialog::DeleteConfirmation => render_delete_dialog(frame, area, app),
        ActiveDialog::CleanupSummary => {
            render_cleanup_summary(frame, area, cleanup_results, app.language())
        }
        ActiveDialog::None => {}
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let language = app.language();
    let parts = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(38), Constraint::Min(10)])
        .split(area);

    let titles = [language.cache_cleanup_tab(), language.space_explorer_tab()]
        .into_iter()
        .map(Line::from)
        .collect::<Vec<_>>();
    let selected = match app.page() {
        Page::CacheCleanup => 0,
        Page::SpaceExplorer => 1,
    };
    let tabs = Tabs::new(titles)
        .select(selected)
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Rgb(129, 214, 219))
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().fg(Color::Rgb(185, 216, 223)))
        .divider(Span::raw(" "))
        .block(
            Block::default()
                .title(format!(" SysClean v{} ", env!("CARGO_PKG_VERSION")))
                .borders(Borders::ALL)
                .border_set(border::ROUNDED),
        );
    frame.render_widget(tabs, parts[0]);

    let status_block = Block::default()
        .title(language.status_title())
        .borders(Borders::ALL)
        .border_set(border::ROUNDED);
    let inner = status_block.inner(parts[1]);
    frame.render_widget(status_block, parts[1]);

    let task_width = if app.task_status.is_some() {
        match inner.width {
            0..=35 => None,
            36..=47 => Some(22),
            _ => Some(30),
        }
    } else {
        None
    };

    let columns = if let Some(width) = task_width {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(10), Constraint::Length(width)])
            .split(inner)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(10)])
            .split(inner)
    };

    let status_widget = Paragraph::new(header_status_text(app))
        .style(Style::default().fg(Color::Rgb(219, 240, 243)))
        .wrap(Wrap { trim: true });
    frame.render_widget(status_widget, columns[0]);

    if let (Some(task), Some(_)) = (app.task_status.as_ref(), task_width) {
        render_task_card(frame, columns[1], task, language);
    }
}

fn header_status_text(app: &App) -> String {
    app.task_status
        .as_ref()
        .map(|task| format!("{} · {}", task.title, task.message))
        .unwrap_or_else(|| app.status_message.clone())
}

fn render_cache_page(frame: &mut Frame, area: Rect, app: &App) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);
    render_cache_table(
        frame,
        columns[0],
        app.cache_items(),
        app.selected_cache_index(),
        app.language(),
    );
    render_cache_details(frame, columns[1], app.selected_cache(), app.language());
}

fn render_cache_table(
    frame: &mut Frame,
    area: Rect,
    items: &[CacheDiscovery],
    selected: usize,
    language: Language,
) {
    let rows = items.iter().map(|item| {
        let reclaimable = format_cache_size(item.reclaimable_bytes, item.size_state, language);
        let total = format_cache_size(Some(item.total_bytes), item.size_state, language);
        let status = cache_status_label(item, language);
        let label = if item.selected {
            format!("[x] {}", item.label)
        } else {
            format!("[ ] {}", item.label)
        };
        Row::new(vec![label, reclaimable, total, status])
    });
    let widths = [
        Constraint::Length(18),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Min(8),
    ];
    let table = Table::new(rows, widths)
        .header(
            Row::new(vec![
                language.cache_column_cache(),
                language.cache_column_reclaimable(),
                language.cache_column_used(),
                language.cache_column_status(),
            ])
            .style(
                Style::default()
                    .fg(Color::Rgb(129, 214, 219))
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(
            Block::default()
                .title(language.cache_cleanup_title())
                .borders(Borders::ALL)
                .border_set(border::ROUNDED),
        )
        .row_highlight_style(Style::default().bg(Color::Rgb(25, 55, 74)))
        .column_spacing(1);
    let mut state = TableState::default();
    if !items.is_empty() {
        state.select(Some(selected));
    }
    frame.render_stateful_widget(table, area, &mut state);
}

fn render_cache_details(
    frame: &mut Frame,
    area: Rect,
    selected: Option<&CacheDiscovery>,
    language: Language,
) {
    let details = if let Some(item) = selected {
        let paths = if item.paths.is_empty() {
            language.docker_cli_only_note().to_string()
        } else {
            item.paths
                .iter()
                .map(|path| format!("• {}", path.display()))
                .collect::<Vec<_>>()
                .join("\n")
        };
        format!(
            "{}: {}\n{}: {}\n{}: {}\n{}: {}\n{}: {}\n{}: {}\n\n{}:\n{}",
            language.detail_name_label(),
            item.label,
            language.detail_description_label(),
            item.description,
            language.detail_status_label(),
            cache_status_label(item, language),
            language.detail_path_count_label(),
            item.paths.len(),
            language.detail_reclaimable_label(),
            format_cache_size(item.reclaimable_bytes, item.size_state, language),
            language.detail_note_label(),
            item.note.as_deref().unwrap_or(language.none_text()),
            language.detail_paths_label(),
            paths
        )
    } else {
        language.no_cache_items().to_string()
    };
    let widget = Paragraph::new(details)
        .block(
            Block::default()
                .title(language.cache_details_title())
                .borders(Borders::ALL)
                .border_set(border::ROUNDED),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, area);
}

fn render_explorer_page(frame: &mut Frame, area: Rect, app: &App, input_mode: InputMode) {
    let language = app.language();
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(8)])
        .split(area);
    let banner = Paragraph::new(format!(
        "{}: {}\n{}: {}{}",
        language.current_path_label(),
        app.current_path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| language.not_loaded().into()),
        language.filter_label(),
        app.explorer_state().filter(),
        if input_mode == InputMode::Filtering {
            language.inputting_suffix()
        } else {
            ""
        }
    ))
    .block(
        Block::default()
            .title(language.path_title())
            .borders(Borders::ALL)
            .border_set(border::ROUNDED),
    )
    .wrap(Wrap { trim: true });
    frame.render_widget(banner, sections[0]);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(sections[1]);
    render_explorer_list(frame, columns[0], app, language);
    render_explorer_details(
        frame,
        columns[1],
        app.explorer_state().selected_entry(),
        language,
    );
}

fn render_explorer_list(frame: &mut Frame, area: Rect, app: &App, language: Language) {
    let visible = app.explorer_state().visible_entries();
    let rows = visible
        .iter()
        .map(|entry| {
            let status = language.scan_state(entry.scan_state);
            Row::new(vec![
                Cell::from(entry.name.clone())
                    .style(Style::default().fg(Color::Rgb(224, 241, 244))),
                Cell::from(format!("{:>10}", format_directory_size(entry, language)))
                    .style(Style::default().fg(Color::Rgb(129, 214, 219))),
                Cell::from(format!("{status:<8}")).style(Style::default().fg(Color::Gray)),
            ])
        })
        .collect::<Vec<_>>();
    let table = Table::new(
        rows,
        [
            Constraint::Fill(1),
            Constraint::Length(12),
            Constraint::Length(10),
        ],
    )
    .header(
        Row::new(vec![
            language.directory_column_name(),
            language.directory_column_size(),
            language.directory_column_status(),
        ])
        .style(
            Style::default()
                .fg(Color::Rgb(129, 214, 219))
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .title(language.space_explorer_title())
            .borders(Borders::ALL)
            .border_set(border::ROUNDED),
    )
    .row_highlight_style(Style::default().bg(Color::Rgb(25, 55, 74)))
    .column_spacing(1);
    let mut state = TableState::default();
    if !visible.is_empty() {
        state.select(Some(app.explorer_state().selected_index()));
    }
    frame.render_stateful_widget(table, area, &mut state);
}

fn render_explorer_details(
    frame: &mut Frame,
    area: Rect,
    entry: Option<&DirectoryEntryInfo>,
    language: Language,
) {
    let text = if let Some(entry) = entry {
        format!(
            "{}: {}\n{}: {}\n{}: {}\n{}: {}\n{}: {}\n{}: {}",
            language.directory_label(),
            entry.name,
            language.path_label(),
            entry.path.display(),
            language.size_label(),
            format_size(entry.size_bytes),
            language.can_enter_label(),
            if entry.can_enter {
                language.yes_text()
            } else {
                language.no_text()
            },
            language.detail_status_label(),
            language.scan_state(entry.scan_state),
            language.remark_label(),
            entry.message.as_deref().unwrap_or(language.none_text()),
        )
    } else {
        language.no_directory_entries().to_string()
    };
    let widget = Paragraph::new(text)
        .block(
            Block::default()
                .title(language.detail_title())
                .borders(Borders::ALL)
                .border_set(border::ROUNDED),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, area);
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App, input_mode: InputMode) {
    let shortcuts_widget = Paragraph::new(footer_shortcuts(app, input_mode))
        .style(Style::default().fg(Color::Rgb(205, 227, 230)))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .title(app.language().shortcuts_title())
                .borders(Borders::ALL)
                .border_set(border::ROUNDED),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(shortcuts_widget, area);
}

fn footer_shortcuts(app: &App, input_mode: InputMode) -> &'static str {
    if input_mode == InputMode::Filtering {
        return app.language().filtering_shortcuts();
    }

    match app.page() {
        Page::CacheCleanup => app.language().cache_shortcuts(),
        Page::SpaceExplorer => app.language().explorer_shortcuts(),
    }
}

fn task_card_supporting_text(task: &BackgroundTaskStatus) -> String {
    task.progress_label_text()
        .unwrap_or_else(|| task.message.clone())
}

fn render_task_card(
    frame: &mut Frame,
    area: Rect,
    task: &BackgroundTaskStatus,
    language: Language,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);
    let card_style = Style::default()
        .bg(Color::Rgb(27, 49, 68))
        .fg(Color::Rgb(219, 240, 243));

    let ratio_text = task
        .progress_ratio()
        .map(|ratio| format!("{:.0}%", ratio * 100.0))
        .unwrap_or_else(|| language.in_progress().to_string());
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            language.background_task(),
            card_style.add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("  {ratio_text}"), card_style),
    ]))
    .style(card_style)
    .alignment(Alignment::Right);
    frame.render_widget(title, rows[0]);

    if let Some(ratio) = task.progress_ratio() {
        let gauge = LineGauge::default()
            .ratio(ratio)
            .filled_style(
                Style::default()
                    .fg(Color::Rgb(129, 214, 219))
                    .bg(Color::Rgb(27, 49, 68)),
            )
            .unfilled_style(
                Style::default()
                    .fg(Color::Rgb(75, 93, 111))
                    .bg(Color::Rgb(27, 49, 68)),
            )
            .label("");
        frame.render_widget(gauge, rows[1]);
    } else {
        let spacer = Paragraph::new("").style(card_style);
        frame.render_widget(spacer, rows[1]);
    }

    let detail = Paragraph::new(task_card_supporting_text(task))
        .style(card_style)
        .alignment(Alignment::Right)
        .wrap(Wrap { trim: true });
    frame.render_widget(detail, rows[2]);
}

fn render_help_dialog(frame: &mut Frame, area: Rect, language: Language) {
    let popup = centered_rect(70, 70, area);
    frame.render_widget(Clear, popup);
    let widget = Paragraph::new(language.help_dialog_text())
        .block(
            Block::default()
                .title(language.help_title())
                .borders(Borders::ALL)
                .border_set(border::ROUNDED),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, popup);
}

fn render_delete_dialog(frame: &mut Frame, area: Rect, app: &App) {
    let language = app.language();
    let popup = centered_rect(60, 50, area);
    frame.render_widget(Clear, popup);
    let content = if let Some(preview) = &app.last_cleanup_preview {
        let items = preview
            .items
            .iter()
            .map(|item| {
                format!(
                    "• {} ({})",
                    item.label,
                    format_cache_size(item.reclaimable_bytes, item.size_state, language)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        language.delete_confirmation_body(&items, &format_size(preview.total_reclaimable_bytes))
    } else {
        language.no_selected_cache_items().to_string()
    };
    let widget = Paragraph::new(content)
        .block(
            Block::default()
                .title(language.delete_confirmation_title())
                .borders(Borders::ALL)
                .border_set(border::ROUNDED),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, popup);
}

fn render_cleanup_summary(
    frame: &mut Frame,
    area: Rect,
    cleanup_results: &[CleanupOutcome],
    language: Language,
) {
    let popup = centered_rect(68, 60, area);
    frame.render_widget(Clear, popup);
    let released: u64 = cleanup_results
        .iter()
        .map(|item| item.bytes_reclaimed)
        .sum();
    let body = cleanup_results
        .iter()
        .map(|item| {
            if item.skipped.is_empty() {
                language.cleanup_result_line(&item.label, &format_size(item.bytes_reclaimed))
            } else {
                language.cleanup_result_line_with_skipped(
                    &item.label,
                    &format_size(item.bytes_reclaimed),
                    item.skipped.len(),
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let text = language.cleanup_summary_body(&format_size(released), &body);
    let widget = Paragraph::new(text)
        .block(
            Block::default()
                .title(language.cleanup_results_title())
                .borders(Borders::ALL)
                .border_set(border::ROUNDED),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, popup);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

pub fn format_size(bytes: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit_index = 0;
    while value >= 1024.0 && unit_index < units.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }
    if unit_index == 0 {
        format!("{bytes} {}", units[unit_index])
    } else {
        format!("{value:.1} {}", units[unit_index])
    }
}

fn format_cache_size(bytes: Option<u64>, state: CacheSizeState, language: Language) -> String {
    match state {
        CacheSizeState::Ready => format_size(bytes.unwrap_or_default()),
        _ => language.cache_size_state(state).into(),
    }
}

fn format_directory_size(entry: &DirectoryEntryInfo, language: Language) -> String {
    match entry.scan_state {
        ScanState::Pending => language.cache_size_state(CacheSizeState::Pending).into(),
        ScanState::Scanning => language.cache_size_state(CacheSizeState::Scanning).into(),
        _ => format_size(entry.size_bytes),
    }
}

fn cache_status_label(item: &CacheDiscovery, language: Language) -> String {
    match item.size_state {
        CacheSizeState::Pending => language.cache_size_state(CacheSizeState::Pending).into(),
        CacheSizeState::Scanning => language.cache_size_state(CacheSizeState::Scanning).into(),
        CacheSizeState::Ready => {
            if item.paths.is_empty() {
                language.cache_status_command_target().into()
            } else if item.total_bytes == 0 {
                language.cache_status_checked_zero().into()
            } else {
                language.cache_status_reclaimable().into()
            }
        }
        CacheSizeState::Unavailable => language
            .cache_size_state(CacheSizeState::Unavailable)
            .into(),
        CacheSizeState::Error => language.cache_size_state(CacheSizeState::Error).into(),
    }
}

#[cfg(test)]
mod tests {
    use super::{InputMode, footer_shortcuts, header_status_text, task_card_supporting_text};
    use crate::app::App;
    use crate::i18n::Language;
    use crate::models::BackgroundTaskStatus;

    #[test]
    fn footer_shortcuts_match_cache_cleanup_page() {
        let app = App::new(Language::En);
        assert_eq!(
            footer_shortcuts(&app, InputMode::Normal),
            "↑↓ Select   Space Toggle   a Select all   r Rescan   d Delete   ? Help   q Quit"
        );
    }

    #[test]
    fn footer_shortcuts_match_space_explorer_page() {
        let mut app = App::new(Language::En);
        app.next_page();
        assert_eq!(
            footer_shortcuts(&app, InputMode::Normal),
            "↑↓ Select   Enter Open   Backspace Up   / Filter   o Open dir   r Rescan   ? Help"
        );
    }

    #[test]
    fn footer_shortcuts_match_filtering_mode() {
        let mut app = App::new(Language::En);
        app.next_page();
        assert_eq!(
            footer_shortcuts(&app, InputMode::Filtering),
            "Type filter   Enter apply   Esc cancel   Backspace delete"
        );
    }

    #[test]
    fn header_status_text_prefers_task_summary_when_task_exists() {
        let mut app = App::new(Language::En);
        app.status_message = "Press ? for help".into();
        app.task_status = Some(BackgroundTaskStatus::new(
            "Directory scan",
            "Calculating sizes",
            true,
        ));

        assert_eq!(
            header_status_text(&app),
            "Directory scan · Calculating sizes"
        );
    }

    #[test]
    fn task_card_supporting_text_prefers_progress_label() {
        let mut status = BackgroundTaskStatus::new("Directory scan", "Calculating sizes", true);
        status.progress_label = Some("Completed 3/6".into());

        assert_eq!(task_card_supporting_text(&status), "Completed 3/6");
    }
}
