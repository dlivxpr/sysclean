use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use sysclean::app::{ActiveDialog, App, Page};
use sysclean::cache_cleaner::{
    CacheDiscovery, CleanupOutcome, SystemCommandRunner, compute_path_size, discover_all_caches,
    execute_cleanup,
};
use sysclean::models::{BackgroundTaskStatus, DirectoryEntryInfo};
use sysclean::persistence::{CacheSnapshot, ScanCache};
use sysclean::platform;
use sysclean::space_explorer::load_directory_entries;
use sysclean::ui::{InputMode, render};

#[derive(Debug)]
enum WorkerMessage {
    CacheScanFinished(Vec<CacheDiscovery>),
    DirectoryScanStarted {
        task_id: u64,
        path: PathBuf,
    },
    DirectoryScanProgress {
        task_id: u64,
        path: PathBuf,
        entries: Vec<DirectoryEntryInfo>,
        scanned: usize,
        total: usize,
        from_cache: bool,
    },
    DirectoryScanFinished {
        task_id: u64,
        path: PathBuf,
        entries: Vec<DirectoryEntryInfo>,
        from_cache: bool,
    },
    CleanupFinished(Vec<CleanupOutcome>),
    TaskFailed(String),
}

fn main() -> Result<()> {
    let mut terminal = init_terminal()?;
    let result = run(&mut terminal);
    restore_terminal(&mut terminal)?;
    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
    let (tx, rx) = mpsc::channel::<WorkerMessage>();
    let mut app = App::default();
    let mut input_mode = InputMode::Normal;
    let mut cleanup_results = Vec::new();
    let mut latest_scan_task_id = 1_u64;

    let home = platform::home_dir()?;
    app.set_current_path(home.clone());
    app.task_status = Some(BackgroundTaskStatus::new(
        "启动",
        "正在扫描缓存和目录",
        true,
    ));

    spawn_cache_scan(tx.clone());
    spawn_directory_scan(tx.clone(), latest_scan_task_id, home, false);

    loop {
        terminal.draw(|frame| render(frame, &app, input_mode, &cleanup_results))?;

        while let Ok(message) = rx.try_recv() {
            handle_worker_message(
                &mut app,
                &mut cleanup_results,
                &mut latest_scan_task_id,
                message,
            );
        }

        if event::poll(Duration::from_millis(100))? {
            let event = event::read()?;
            if let Event::Key(key) = event
                && key.kind == KeyEventKind::Press
                && handle_key_event(
                    &mut app,
                    &mut input_mode,
                    &mut cleanup_results,
                    &tx,
                    &mut latest_scan_task_id,
                    key,
                )?
            {
                break;
            }
        }
    }

    Ok(())
}

fn handle_worker_message(
    app: &mut App,
    cleanup_results: &mut Vec<CleanupOutcome>,
    latest_scan_task_id: &mut u64,
    message: WorkerMessage,
) {
    match message {
        WorkerMessage::CacheScanFinished(items) => {
            app.set_cache_items(items);
            app.status_message = "缓存扫描完成".into();
            if matches!(app.page(), Page::CacheCleanup) {
                app.task_status = None;
            }
        }
        WorkerMessage::DirectoryScanStarted { task_id, path } => {
            *latest_scan_task_id = task_id;
            if app.current_path().is_none() {
                app.set_current_path(path);
            }
            app.task_status = Some(BackgroundTaskStatus::new(
                "目录扫描",
                "正在统计目录大小",
                true,
            ));
        }
        WorkerMessage::DirectoryScanProgress {
            task_id,
            path,
            entries,
            scanned,
            total,
            from_cache,
        } => {
            if task_id != *latest_scan_task_id {
                return;
            }
            if app
                .current_path()
                .map(|current| current != &path)
                .unwrap_or(true)
            {
                app.set_current_path(path);
            }
            app.explorer_state_mut().set_entries(entries);
            let mut task = BackgroundTaskStatus::new(
                "目录扫描",
                if from_cache {
                    "已加载缓存结果"
                } else {
                    "正在渐进更新列表"
                },
                true,
            );
            task.progress_current = scanned;
            task.progress_total = total;
            app.task_status = Some(task);
        }
        WorkerMessage::DirectoryScanFinished {
            task_id,
            path,
            entries,
            from_cache,
        } => {
            if task_id != *latest_scan_task_id {
                return;
            }
            if app
                .current_path()
                .map(|current| current != &path)
                .unwrap_or(true)
            {
                app.set_current_path(path);
            }
            app.explorer_state_mut().set_entries(entries);
            app.task_status = None;
            app.status_message = if from_cache {
                "目录结果来自本地缓存，按 r 可强制重扫".into()
            } else {
                "目录扫描完成".into()
            };
        }
        WorkerMessage::CleanupFinished(results) => {
            *cleanup_results = results;
            app.task_status = None;
            app.status_message = "缓存清理已完成".into();
            app.last_cleanup_preview = None;
            app.show_cleanup_summary();
        }
        WorkerMessage::TaskFailed(message) => {
            app.task_status = None;
            app.status_message = message;
        }
    }
}

fn handle_key_event(
    app: &mut App,
    input_mode: &mut InputMode,
    cleanup_results: &mut Vec<CleanupOutcome>,
    tx: &Sender<WorkerMessage>,
    latest_scan_task_id: &mut u64,
    key: KeyEvent,
) -> Result<bool> {
    if *input_mode == InputMode::Filtering {
        return handle_filter_input(app, input_mode, key);
    }

    match app.active_dialog() {
        ActiveDialog::Help => {
            if matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('?')) {
                app.close_dialog();
            }
            return Ok(false);
        }
        ActiveDialog::DeleteConfirmation => {
            match key.code {
                KeyCode::Esc => app.close_dialog(),
                KeyCode::Enter => {
                    app.task_status = Some(BackgroundTaskStatus::new(
                        "缓存清理",
                        "正在删除所选缓存",
                        true,
                    ));
                    spawn_cleanup(tx.clone(), app.cache_items().to_vec());
                }
                _ => {}
            }
            return Ok(false);
        }
        ActiveDialog::CleanupSummary => {
            if matches!(key.code, KeyCode::Esc | KeyCode::Enter) {
                cleanup_results.clear();
                app.close_dialog();
            }
            return Ok(false);
        }
        ActiveDialog::None => {}
    }

    match key.code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Char('?') => app.toggle_help(),
        KeyCode::Tab | KeyCode::Right | KeyCode::Left => app.next_page(),
        KeyCode::Esc | KeyCode::Char('c') => {
            app.task_status = None;
            app.status_message = "已取消当前任务显示，旧结果会被忽略".into();
        }
        _ => match app.page() {
            Page::CacheCleanup => handle_cache_keys(app, tx, key),
            Page::SpaceExplorer => {
                handle_explorer_keys(app, input_mode, tx, latest_scan_task_id, key)?
            }
        },
    }

    Ok(false)
}

fn handle_cache_keys(app: &mut App, tx: &Sender<WorkerMessage>, key: KeyEvent) {
    match key.code {
        KeyCode::Up => app.select_previous_cache(),
        KeyCode::Down => app.select_next_cache(),
        KeyCode::Char(' ') => app.toggle_selected_cache(),
        KeyCode::Char('a') => app.toggle_all_caches(),
        KeyCode::Char('r') => {
            app.task_status = Some(BackgroundTaskStatus::new(
                "缓存扫描",
                "重新发现缓存目标",
                true,
            ));
            spawn_cache_scan(tx.clone());
        }
        KeyCode::Char('d') => app.open_delete_confirmation(),
        _ => {}
    }
}

fn handle_explorer_keys(
    app: &mut App,
    input_mode: &mut InputMode,
    tx: &Sender<WorkerMessage>,
    latest_scan_task_id: &mut u64,
    key: KeyEvent,
) -> Result<()> {
    match key.code {
        KeyCode::Up => app.explorer_state_mut().select_previous(),
        KeyCode::Down => app.explorer_state_mut().select_next(),
        KeyCode::Home => app.explorer_state_mut().select_first(),
        KeyCode::End => app.explorer_state_mut().select_last(),
        KeyCode::PageUp => app.explorer_state_mut().page_up(8),
        KeyCode::PageDown => app.explorer_state_mut().page_down(8),
        KeyCode::Char('/') => {
            *input_mode = InputMode::Filtering;
            app.filter_input.clear();
            app.status_message = "输入筛选关键字，按 Enter 应用".into();
        }
        KeyCode::Char('o') => {
            if let Some(entry) = app.explorer_state().selected_entry() {
                platform::open_in_explorer(&entry.path)?;
                app.status_message = format!("已在资源管理器中打开 {}", entry.path.display());
            }
        }
        KeyCode::Char('r') => {
            if let Some(path) = app.current_path().cloned() {
                *latest_scan_task_id += 1;
                spawn_directory_scan(tx.clone(), *latest_scan_task_id, path, true);
            }
        }
        KeyCode::Backspace => {
            app.pop_directory();
            if let Some(path) = app.current_path().cloned() {
                *latest_scan_task_id += 1;
                spawn_directory_scan(tx.clone(), *latest_scan_task_id, path, false);
            }
        }
        KeyCode::Enter => {
            if let Some(entry) = app.explorer_state().selected_entry()
                && entry.can_enter
            {
                let next = entry.path.clone();
                app.push_directory(next.clone());
                *latest_scan_task_id += 1;
                spawn_directory_scan(tx.clone(), *latest_scan_task_id, next, false);
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_filter_input(app: &mut App, input_mode: &mut InputMode, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            *input_mode = InputMode::Normal;
            app.status_message = "已退出过滤模式".into();
        }
        KeyCode::Enter => {
            let filter = app.filter_input.clone();
            app.explorer_state_mut().set_filter(filter);
            *input_mode = InputMode::Normal;
            app.status_message = if app.filter_input.is_empty() {
                "已清空过滤条件".into()
            } else {
                format!("过滤: {}", app.filter_input)
            };
        }
        KeyCode::Backspace => {
            app.filter_input.pop();
            let filter = app.filter_input.clone();
            app.explorer_state_mut().set_filter(filter);
        }
        KeyCode::Char(ch) => {
            app.filter_input.push(ch);
            let filter = app.filter_input.clone();
            app.explorer_state_mut().set_filter(filter);
        }
        _ => {}
    }
    Ok(false)
}

fn spawn_cache_scan(tx: Sender<WorkerMessage>) {
    thread::spawn(move || {
        let runner = SystemCommandRunner;
        let result = (|| -> Result<Vec<CacheDiscovery>> {
            let home = platform::home_dir()?;
            let local = platform::local_app_data_dir()?;
            discover_all_caches(&runner, home, local)
        })();
        match result {
            Ok(items) => {
                let _ = tx.send(WorkerMessage::CacheScanFinished(items));
            }
            Err(error) => {
                let _ = tx.send(WorkerMessage::TaskFailed(format!("缓存扫描失败: {error}")));
            }
        }
    });
}

fn spawn_directory_scan(
    tx: Sender<WorkerMessage>,
    task_id: u64,
    path: PathBuf,
    force_refresh: bool,
) {
    thread::spawn(move || {
        let _ = tx.send(WorkerMessage::DirectoryScanStarted {
            task_id,
            path: path.clone(),
        });

        let cache = match platform::app_cache_file() {
            Ok(file_path) => ScanCache::new(file_path),
            Err(error) => {
                let _ = tx.send(WorkerMessage::TaskFailed(format!(
                    "初始化扫描缓存失败: {error}"
                )));
                return;
            }
        };

        if !force_refresh {
            match load_directory_entries(&path, &cache) {
                Ok((entries, true)) => {
                    let total = entries.len();
                    let _ = tx.send(WorkerMessage::DirectoryScanProgress {
                        task_id,
                        path: path.clone(),
                        entries: entries.clone(),
                        scanned: total,
                        total,
                        from_cache: true,
                    });
                    let _ = tx.send(WorkerMessage::DirectoryScanFinished {
                        task_id,
                        path,
                        entries,
                        from_cache: true,
                    });
                    return;
                }
                Ok(_) => {}
                Err(error) => {
                    let _ = tx.send(WorkerMessage::TaskFailed(format!("目录扫描失败: {error}")));
                    return;
                }
            }
        }

        let read_dir = match std::fs::read_dir(&path) {
            Ok(read_dir) => read_dir,
            Err(error) => {
                let _ = tx.send(WorkerMessage::TaskFailed(format!(
                    "无法读取目录 {}: {error}",
                    path.display()
                )));
                return;
            }
        };

        let mut directories = Vec::new();
        for entry in read_dir.flatten() {
            if let Ok(metadata) = std::fs::symlink_metadata(entry.path())
                && (metadata.is_dir() || metadata.file_type().is_symlink())
            {
                directories.push(entry.path());
            }
        }

        let total = directories.len();
        let mut built = Vec::new();
        for (index, child_path) in directories.into_iter().enumerate() {
            let name = child_path
                .file_name()
                .and_then(|item| item.to_str())
                .unwrap_or("<未知>")
                .to_string();
            let entry = match std::fs::symlink_metadata(&child_path) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    DirectoryEntryInfo::new_skipped(
                        name,
                        child_path.clone(),
                        "符号链接或 junction 已跳过",
                    )
                }
                Ok(_) => match compute_path_size(&child_path) {
                    Ok(size) => DirectoryEntryInfo::new_ready(name, child_path.clone(), size, true),
                    Err(error) => {
                        DirectoryEntryInfo::new_error(name, child_path.clone(), error.to_string())
                    }
                },
                Err(error) => {
                    DirectoryEntryInfo::new_error(name, child_path.clone(), error.to_string())
                }
            };
            built.push(entry);
            built.sort_by(|left, right| {
                right
                    .size_bytes
                    .cmp(&left.size_bytes)
                    .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            });
            let _ = tx.send(WorkerMessage::DirectoryScanProgress {
                task_id,
                path: path.clone(),
                entries: built.clone(),
                scanned: index + 1,
                total,
                from_cache: false,
            });
        }

        let snapshot = CacheSnapshot {
            path: path.clone(),
            captured_at: chrono::Utc::now(),
            entries: built.clone(),
        };
        let _ = cache.save_snapshot(&snapshot);
        let _ = tx.send(WorkerMessage::DirectoryScanFinished {
            task_id,
            path,
            entries: built,
            from_cache: false,
        });
    });
}

fn spawn_cleanup(tx: Sender<WorkerMessage>, items: Vec<CacheDiscovery>) {
    thread::spawn(move || {
        let runner = SystemCommandRunner;
        let results = execute_cleanup(&items, &runner);
        let _ = tx.send(WorkerMessage::CleanupFinished(results));
    });
}

fn init_terminal() -> Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
