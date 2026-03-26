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
        ActiveDialog::Help => render_help_dialog(frame, area),
        ActiveDialog::DeleteConfirmation => render_delete_dialog(frame, area, app),
        ActiveDialog::CleanupSummary => render_cleanup_summary(frame, area, cleanup_results),
        ActiveDialog::None => {}
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let parts = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(38), Constraint::Min(10)])
        .split(area);

    let titles = ["Cache Cleanup", "Space Explorer"]
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
        .title(" 状态 ")
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
        render_task_card(frame, columns[1], task);
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
    );
    render_cache_details(frame, columns[1], app.selected_cache());
}

fn render_cache_table(frame: &mut Frame, area: Rect, items: &[CacheDiscovery], selected: usize) {
    let rows = items.iter().map(|item| {
        let reclaimable = format_cache_size(item.reclaimable_bytes, item.size_state);
        let total = format_cache_size(Some(item.total_bytes), item.size_state);
        let status = cache_status_label(item);
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
            Row::new(vec!["缓存", "可释放", "占用", "状态"]).style(
                Style::default()
                    .fg(Color::Rgb(129, 214, 219))
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(
            Block::default()
                .title(" 缓存清理 ")
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

fn render_cache_details(frame: &mut Frame, area: Rect, selected: Option<&CacheDiscovery>) {
    let details = if let Some(item) = selected {
        let paths = if item.paths.is_empty() {
            "此目标通过 docker CLI 清理，不直接暴露删除任意路径。".to_string()
        } else {
            item.paths
                .iter()
                .map(|path| format!("• {}", path.display()))
                .collect::<Vec<_>>()
                .join("\n")
        };
        format!(
            "名称: {}\n说明: {}\n状态: {}\n路径数: {}\n可释放: {}\n备注: {}\n\n路径:\n{}",
            item.label,
            item.description,
            cache_status_label(item),
            item.paths.len(),
            format_cache_size(item.reclaimable_bytes, item.size_state),
            item.note.as_deref().unwrap_or("无"),
            paths
        )
    } else {
        "暂无缓存项。".to_string()
    };
    let widget = Paragraph::new(details)
        .block(
            Block::default()
                .title(" 明细 ")
                .borders(Borders::ALL)
                .border_set(border::ROUNDED),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, area);
}

fn render_explorer_page(frame: &mut Frame, area: Rect, app: &App, input_mode: InputMode) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(8)])
        .split(area);
    let banner = Paragraph::new(format!(
        "当前位置: {}\n筛选: {}{}",
        app.current_path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<未加载>".into()),
        app.explorer_state().filter(),
        if input_mode == InputMode::Filtering {
            "  (输入中)"
        } else {
            ""
        }
    ))
    .block(
        Block::default()
            .title(" 路径 ")
            .borders(Borders::ALL)
            .border_set(border::ROUNDED),
    )
    .wrap(Wrap { trim: true });
    frame.render_widget(banner, sections[0]);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(sections[1]);
    render_explorer_list(frame, columns[0], app);
    render_explorer_details(frame, columns[1], app.explorer_state().selected_entry());
}

fn render_explorer_list(frame: &mut Frame, area: Rect, app: &App) {
    let visible = app.explorer_state().visible_entries();
    let rows = visible
        .iter()
        .map(|entry| {
            let status = match entry.scan_state {
                ScanState::Ready => "就绪",
                ScanState::Cached => "缓存",
                ScanState::Scanning => "扫描中",
                ScanState::Pending => "待扫描",
                ScanState::Skipped => "跳过",
                ScanState::Error => "失败",
            };
            Row::new(vec![
                Cell::from(entry.name.clone())
                    .style(Style::default().fg(Color::Rgb(224, 241, 244))),
                Cell::from(format!("{:>10}", format_directory_size(entry)))
                    .style(Style::default().fg(Color::Rgb(129, 214, 219))),
                Cell::from(format!("{status:<6}")).style(Style::default().fg(Color::Gray)),
            ])
        })
        .collect::<Vec<_>>();
    let table = Table::new(
        rows,
        [
            Constraint::Fill(1),
            Constraint::Length(12),
            Constraint::Length(8),
        ],
    )
    .header(
        Row::new(vec!["目录", "大小", "状态"]).style(
            Style::default()
                .fg(Color::Rgb(129, 214, 219))
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .title(" 目录分析 ")
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

fn render_explorer_details(frame: &mut Frame, area: Rect, entry: Option<&DirectoryEntryInfo>) {
    let text = if let Some(entry) = entry {
        format!(
            "目录: {}\n路径: {}\n大小: {}\n可进入: {}\n状态: {:?}\n备注: {}",
            entry.name,
            entry.path.display(),
            format_size(entry.size_bytes),
            if entry.can_enter { "是" } else { "否" },
            entry.scan_state,
            entry.message.as_deref().unwrap_or("无"),
        )
    } else {
        "没有可显示的目录。".to_string()
    };
    let widget = Paragraph::new(text)
        .block(
            Block::default()
                .title(" 详情 ")
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
                .title(" 常用快捷键 ")
                .borders(Borders::ALL)
                .border_set(border::ROUNDED),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(shortcuts_widget, area);
}

fn footer_shortcuts(app: &App, input_mode: InputMode) -> &'static str {
    if input_mode == InputMode::Filtering {
        return "输入关键字   Enter 应用   Esc 取消   Backspace 删除";
    }

    match app.page() {
        Page::CacheCleanup => "↑↓ 选择   Space 勾选   a 全选   r 重扫   d 删除   ? 帮助   q 退出",
        Page::SpaceExplorer => {
            "↑↓ 选择   Enter 进入   Backspace 返回   / 过滤   o 打开目录   r 重扫   ? 帮助"
        }
    }
}

fn task_card_supporting_text(task: &BackgroundTaskStatus) -> String {
    task.progress_label_text()
        .unwrap_or_else(|| task.message.clone())
}

fn render_task_card(frame: &mut Frame, area: Rect, task: &BackgroundTaskStatus) {
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
        .unwrap_or_else(|| "进行中".to_string());
    let title = Paragraph::new(Line::from(vec![
        Span::styled("后台任务", card_style.add_modifier(Modifier::BOLD)),
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

fn render_help_dialog(frame: &mut Frame, area: Rect) {
    let popup = centered_rect(70, 70, area);
    frame.render_widget(Clear, popup);
    let text = "全局快捷键\nTab / ←→ 切换工作区\nq 退出\n? 打开帮助\nEsc 关闭弹窗或取消输入\n\n缓存清理\n↑↓ 选择缓存\nSpace 勾选\nA 全选或反选\nR 重扫缓存\nD 打开删除确认\n路径会先显示，大小会后台逐项更新\n\n目录分析\n↑↓ 选择目录\nEnter 进入目录\nBackspace 返回上级\nHome / End 跳到首尾\nPgUp / PgDn 快速翻页\n/ 进入过滤\nO 打开资源管理器\n进入目录后会先展示骨架，再边扫边重排";
    let widget = Paragraph::new(text)
        .block(
            Block::default()
                .title(" 帮助 ")
                .borders(Borders::ALL)
                .border_set(border::ROUNDED),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, popup);
}

fn render_delete_dialog(frame: &mut Frame, area: Rect, app: &App) {
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
                    format_cache_size(item.reclaimable_bytes, item.size_state)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "即将删除以下缓存项:\n{}\n\n预计释放: {}\n\n按 Enter 确认, Esc 取消。",
            items,
            format_size(preview.total_reclaimable_bytes)
        )
    } else {
        "没有选中的缓存项。".to_string()
    };
    let widget = Paragraph::new(content)
        .block(
            Block::default()
                .title(" 删除确认 ")
                .borders(Borders::ALL)
                .border_set(border::ROUNDED),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, popup);
}

fn render_cleanup_summary(frame: &mut Frame, area: Rect, cleanup_results: &[CleanupOutcome]) {
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
                format!(
                    "• {}: 释放 {}",
                    item.label,
                    format_size(item.bytes_reclaimed)
                )
            } else {
                format!(
                    "• {}: 释放 {} | 跳过 {} 项",
                    item.label,
                    format_size(item.bytes_reclaimed),
                    item.skipped.len()
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let text = format!(
        "本次清理完成。\n总释放: {}\n\n{}\n\n按 Esc 或 Enter 关闭。",
        format_size(released),
        body
    );
    let widget = Paragraph::new(text)
        .block(
            Block::default()
                .title(" 清理结果 ")
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

fn format_cache_size(bytes: Option<u64>, state: CacheSizeState) -> String {
    match state {
        CacheSizeState::Pending => "待计算".into(),
        CacheSizeState::Scanning => "计算中".into(),
        CacheSizeState::Unavailable => "不可用".into(),
        CacheSizeState::Error => "失败".into(),
        CacheSizeState::Ready => format_size(bytes.unwrap_or_default()),
    }
}

fn format_directory_size(entry: &DirectoryEntryInfo) -> String {
    match entry.scan_state {
        ScanState::Pending => "待计算".into(),
        ScanState::Scanning => "计算中".into(),
        _ => format_size(entry.size_bytes),
    }
}

fn cache_status_label(item: &CacheDiscovery) -> String {
    match item.size_state {
        CacheSizeState::Pending => "待计算".into(),
        CacheSizeState::Scanning => "计算中".into(),
        CacheSizeState::Ready => {
            if item.paths.is_empty() {
                "命令型目标".into()
            } else if item.total_bytes == 0 {
                "已检查 0B".into()
            } else {
                "可清理".into()
            }
        }
        CacheSizeState::Unavailable => "不可用".into(),
        CacheSizeState::Error => "失败".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::{InputMode, footer_shortcuts, header_status_text, task_card_supporting_text};
    use crate::app::App;
    use crate::models::BackgroundTaskStatus;

    #[test]
    fn footer_shortcuts_match_cache_cleanup_page() {
        let app = App::default();
        assert_eq!(
            footer_shortcuts(&app, InputMode::Normal),
            "↑↓ 选择   Space 勾选   a 全选   r 重扫   d 删除   ? 帮助   q 退出"
        );
    }

    #[test]
    fn footer_shortcuts_match_space_explorer_page() {
        let mut app = App::default();
        app.next_page();
        assert_eq!(
            footer_shortcuts(&app, InputMode::Normal),
            "↑↓ 选择   Enter 进入   Backspace 返回   / 过滤   o 打开目录   r 重扫   ? 帮助"
        );
    }

    #[test]
    fn footer_shortcuts_match_filtering_mode() {
        let mut app = App::default();
        app.next_page();
        assert_eq!(
            footer_shortcuts(&app, InputMode::Filtering),
            "输入关键字   Enter 应用   Esc 取消   Backspace 删除"
        );
    }

    #[test]
    fn header_status_text_prefers_task_summary_when_task_exists() {
        let mut app = App::default();
        app.status_message = "按 ? 查看帮助".into();
        app.task_status = Some(BackgroundTaskStatus::new("目录扫描", "正在计算大小", true));

        assert_eq!(header_status_text(&app), "目录扫描 · 正在计算大小");
    }

    #[test]
    fn task_card_supporting_text_prefers_progress_label() {
        let mut status = BackgroundTaskStatus::new("目录扫描", "正在计算大小", true);
        status.progress_label = Some("已完成 3/6".into());

        assert_eq!(task_card_supporting_text(&status), "已完成 3/6");
    }
}
