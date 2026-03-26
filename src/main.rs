use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::{Duration, Instant};

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
    CacheDiscovery, CacheSizeState, CleanupOutcome, CleanupProgress, SUPPORTED_CACHE_TARGETS,
    SystemCommandRunner, compute_path_size, discover_cache_metadata, execute_cleanup_with_progress,
    populate_cache_size,
};
use sysclean::models::{BackgroundTaskStatus, DirectoryEntryInfo, ScanState};
use sysclean::persistence::{CacheSnapshot, ScanCache};
use sysclean::platform;
use sysclean::space_explorer::{
    DIRECTORY_UPDATE_BATCH_SIZE, DIRECTORY_UPDATE_THROTTLE, discover_directory_skeleton,
    load_directory_entries, recommended_worker_count,
};
use sysclean::ui::{InputMode, render};

#[derive(Debug)]
enum WorkerMessage {
    CacheDiscoveryStarted {
        task_id: u64,
        total: usize,
    },
    CacheItemDiscovered {
        task_id: u64,
        item: CacheDiscovery,
        discovered: usize,
        total: usize,
    },
    CacheItemSized {
        task_id: u64,
        item: CacheDiscovery,
        completed: usize,
        total: usize,
    },
    CacheScanFinished {
        task_id: u64,
        total: usize,
    },
    DirectoryScanStarted {
        task_id: u64,
        path: PathBuf,
    },
    DirectoryEntriesDiscovered {
        task_id: u64,
        path: PathBuf,
        entries: Vec<DirectoryEntryInfo>,
        total: usize,
        from_cache: bool,
    },
    DirectoryEntriesUpdated {
        task_id: u64,
        path: PathBuf,
        entries: Vec<DirectoryEntryInfo>,
        completed: usize,
        total: usize,
        from_cache: bool,
    },
    DirectoryScanFinished {
        task_id: u64,
        path: PathBuf,
        entries: Vec<DirectoryEntryInfo>,
        from_cache: bool,
    },
    CleanupStarted {
        total_paths: usize,
        total_bytes: Option<u64>,
    },
    CleanupProgress(CleanupProgress),
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
    let mut latest_directory_task_id = 1_u64;
    let mut latest_cache_task_id = 1_u64;

    let home = platform::home_dir()?;
    app.set_current_path(home.clone());
    app.task_status = Some(BackgroundTaskStatus::new(
        "启动",
        "正在准备缓存与目录扫描",
        true,
    ));

    spawn_cache_scan(tx.clone(), latest_cache_task_id);
    spawn_directory_scan(tx.clone(), latest_directory_task_id, home, false);

    loop {
        terminal.draw(|frame| render(frame, &app, input_mode, &cleanup_results))?;

        while let Ok(message) = rx.try_recv() {
            handle_worker_message(
                &mut app,
                &mut cleanup_results,
                &mut latest_directory_task_id,
                &mut latest_cache_task_id,
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
                    &mut latest_directory_task_id,
                    &mut latest_cache_task_id,
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
    latest_directory_task_id: &mut u64,
    latest_cache_task_id: &mut u64,
    message: WorkerMessage,
) {
    match message {
        WorkerMessage::CacheDiscoveryStarted { task_id, total } => {
            *latest_cache_task_id = task_id;
            app.task_status = Some(progress_status(
                "缓存扫描",
                format!("已发现 0/{total} 个缓存目标"),
                0,
                total,
            ));
        }
        WorkerMessage::CacheItemDiscovered {
            task_id,
            item,
            discovered,
            total,
        } => {
            if task_id != *latest_cache_task_id {
                return;
            }
            app.upsert_cache_item(item);
            let mut status = progress_status(
                "缓存扫描",
                format!("已发现 {discovered}/{total} 个缓存目标，正在计算大小"),
                discovered,
                total,
            );
            status.progress_label = Some(format!("已发现 {discovered}/{total}"));
            app.task_status = Some(status);
        }
        WorkerMessage::CacheItemSized {
            task_id,
            item,
            completed,
            total,
        } => {
            if task_id != *latest_cache_task_id {
                return;
            }
            app.upsert_cache_item(item);
            let mut status = progress_status(
                "缓存扫描",
                format!("已发现 {total}/{total} 个缓存目标，正在计算 {completed}/{total} 个大小"),
                completed,
                total,
            );
            status.progress_label = Some(format!("已完成 {completed}/{total}"));
            app.task_status = Some(status);
        }
        WorkerMessage::CacheScanFinished { task_id, total } => {
            if task_id != *latest_cache_task_id {
                return;
            }
            app.status_message = if total == 0 {
                "未发现可扫描的缓存目标".into()
            } else {
                "缓存路径已全部展示，大小计算完成".into()
            };
            if matches!(app.page(), Page::CacheCleanup) {
                app.task_status = None;
            }
        }
        WorkerMessage::DirectoryScanStarted { task_id, path } => {
            *latest_directory_task_id = task_id;
            app.task_status = Some(progress_status(
                "目录扫描",
                format!("正在枚举 {} 的直接子目录", path.display()),
                0,
                0,
            ));
        }
        WorkerMessage::DirectoryEntriesDiscovered {
            task_id,
            path,
            entries,
            total,
            from_cache,
        } => {
            if task_id != *latest_directory_task_id {
                return;
            }
            if app
                .current_path()
                .map(|current| current != &path)
                .unwrap_or(true)
            {
                return;
            }
            app.explorer_state_mut().set_entries(entries);
            let mut status = progress_status(
                "目录扫描",
                if from_cache {
                    format!("已从缓存加载 {total} 个子目录")
                } else {
                    format!("已枚举 {total} 个子目录，正在逐项计算大小")
                },
                0,
                total,
            );
            status.determinate = !from_cache && total > 0;
            status.progress_label = Some(if from_cache {
                format!("缓存命中 {total} 项")
            } else {
                format!("已完成 0/{total}")
            });
            app.task_status = Some(status);
        }
        WorkerMessage::DirectoryEntriesUpdated {
            task_id,
            path,
            entries,
            completed,
            total,
            from_cache,
        } => {
            if task_id != *latest_directory_task_id {
                return;
            }
            if app
                .current_path()
                .map(|current| current != &path)
                .unwrap_or(true)
            {
                return;
            }
            let mut current_entries = app.explorer_state().entries().to_vec();
            for entry in entries {
                upsert_directory_entry(&mut current_entries, entry);
            }
            app.explorer_state_mut().set_entries(current_entries);
            let mut status = progress_status(
                "目录扫描",
                if from_cache {
                    format!("缓存结果已加载 {completed}/{total}")
                } else {
                    format!("已枚举 {total} 个子目录，正在计算 {completed}/{total} 个大小")
                },
                completed,
                total,
            );
            status.progress_label = Some(format!("已完成 {completed}/{total}"));
            app.task_status = Some(status);
        }
        WorkerMessage::DirectoryScanFinished {
            task_id,
            path,
            entries,
            from_cache,
        } => {
            if task_id != *latest_directory_task_id {
                return;
            }
            if app
                .current_path()
                .map(|current| current != &path)
                .unwrap_or(true)
            {
                return;
            }
            app.explorer_state_mut().set_entries(entries);
            app.task_status = None;
            app.status_message = if from_cache {
                "目录骨架已从缓存加载，按 r 可强制重扫".into()
            } else {
                "目录骨架已展示，大小已渐进更新完成".into()
            };
        }
        WorkerMessage::CleanupFinished(results) => {
            *cleanup_results = results;
            app.task_status = None;
            app.status_message = "缓存清理已完成".into();
            app.last_cleanup_preview = None;
            app.show_cleanup_summary();
        }
        WorkerMessage::CleanupStarted {
            total_paths,
            total_bytes,
        } => {
            app.task_status = Some(cleanup_status(
                "正在准备删除缓存".into(),
                0,
                total_paths,
                0,
                total_bytes,
            ));
        }
        WorkerMessage::CleanupProgress(progress) => {
            app.task_status = Some(cleanup_status(
                format!(
                    "正在清理 {} ({}/{})",
                    progress.current_label, progress.completed_paths, progress.total_paths
                ),
                progress.completed_paths,
                progress.total_paths,
                progress.completed_bytes,
                progress.total_bytes,
            ));
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
    latest_directory_task_id: &mut u64,
    latest_cache_task_id: &mut u64,
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
                    app.task_status = Some(progress_status(
                        "缓存清理",
                        "正在删除已完成大小计算的缓存项".into(),
                        0,
                        0,
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
            Page::CacheCleanup => handle_cache_keys(app, tx, latest_cache_task_id, key),
            Page::SpaceExplorer => {
                handle_explorer_keys(app, input_mode, tx, latest_directory_task_id, key)?
            }
        },
    }

    Ok(false)
}

fn handle_cache_keys(
    app: &mut App,
    tx: &Sender<WorkerMessage>,
    latest_cache_task_id: &mut u64,
    key: KeyEvent,
) {
    match key.code {
        KeyCode::Up => app.select_previous_cache(),
        KeyCode::Down => app.select_next_cache(),
        KeyCode::Char(' ') => app.toggle_selected_cache(),
        KeyCode::Char('a') => app.toggle_all_caches(),
        KeyCode::Char('r') => {
            *latest_cache_task_id += 1;
            app.task_status = Some(progress_status(
                "缓存扫描",
                "正在重新发现缓存路径".into(),
                0,
                SUPPORTED_CACHE_TARGETS.len(),
            ));
            spawn_cache_scan(tx.clone(), *latest_cache_task_id);
        }
        KeyCode::Char('d') => app.open_delete_confirmation(),
        _ => {}
    }
}

fn handle_explorer_keys(
    app: &mut App,
    input_mode: &mut InputMode,
    tx: &Sender<WorkerMessage>,
    latest_directory_task_id: &mut u64,
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
                *latest_directory_task_id += 1;
                spawn_directory_scan(tx.clone(), *latest_directory_task_id, path, true);
            }
        }
        KeyCode::Backspace => {
            app.pop_directory();
            if let Some(path) = app.current_path().cloned() {
                *latest_directory_task_id += 1;
                spawn_directory_scan(tx.clone(), *latest_directory_task_id, path, false);
            }
        }
        KeyCode::Enter => {
            if let Some(entry) = app.explorer_state().selected_entry()
                && entry.can_enter
            {
                let next = entry.path.clone();
                app.push_directory(next.clone());
                app.explorer_state_mut().set_entries(Vec::new());
                app.status_message = format!("已进入 {}，正在枚举子目录", next.display());
                *latest_directory_task_id += 1;
                spawn_directory_scan(tx.clone(), *latest_directory_task_id, next, false);
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

fn spawn_cache_scan(tx: Sender<WorkerMessage>, task_id: u64) {
    thread::spawn(move || {
        let runner = SystemCommandRunner;
        let home = match platform::home_dir() {
            Ok(path) => path,
            Err(error) => {
                let _ = tx.send(WorkerMessage::TaskFailed(format!("缓存扫描失败: {error}")));
                return;
            }
        };
        let local = match platform::local_app_data_dir() {
            Ok(path) => path,
            Err(error) => {
                let _ = tx.send(WorkerMessage::TaskFailed(format!("缓存扫描失败: {error}")));
                return;
            }
        };

        let total = SUPPORTED_CACHE_TARGETS.len();
        let _ = tx.send(WorkerMessage::CacheDiscoveryStarted { task_id, total });
        let (size_tx, size_rx) = mpsc::channel::<CacheDiscovery>();
        let mut spawned_jobs = 0_usize;
        let mut completed_sizes = 0_usize;

        for (discovered, kind) in SUPPORTED_CACHE_TARGETS.iter().copied().enumerate() {
            let item = match discover_cache_metadata(
                kind,
                &runner,
                Some(home.clone()),
                Some(local.clone()),
            ) {
                Ok(item) => item,
                Err(error) => {
                    let _ = tx.send(WorkerMessage::TaskFailed(format!("缓存扫描失败: {error}")));
                    return;
                }
            };

            let mut discovered_item = item.clone();
            if discovered_item.available && discovered_item.size_state == CacheSizeState::Pending {
                discovered_item.size_state = CacheSizeState::Scanning;
            }
            let _ = tx.send(WorkerMessage::CacheItemDiscovered {
                task_id,
                item: discovered_item,
                discovered: discovered + 1,
                total,
            });

            if item.kind == sysclean::cache_cleaner::CacheTargetKind::Docker
                || !item.available
                || item.size_state == CacheSizeState::Unavailable
            {
                completed_sizes += 1;
                let _ = tx.send(WorkerMessage::CacheItemSized {
                    task_id,
                    item,
                    completed: completed_sizes,
                    total,
                });
                continue;
            }

            spawned_jobs += 1;
            let size_tx = size_tx.clone();
            thread::spawn(move || {
                let mut sized_item = item;
                let _ = populate_cache_size(&mut sized_item);
                let _ = size_tx.send(sized_item);
            });
        }

        drop(size_tx);

        for sized_item in size_rx.iter().take(spawned_jobs) {
            completed_sizes += 1;
            let _ = tx.send(WorkerMessage::CacheItemSized {
                task_id,
                item: sized_item,
                completed: completed_sizes,
                total,
            });
        }

        let _ = tx.send(WorkerMessage::CacheScanFinished { task_id, total });
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
                    let _ = tx.send(WorkerMessage::DirectoryEntriesDiscovered {
                        task_id,
                        path: path.clone(),
                        entries: entries.clone(),
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

        let mut entries = match discover_directory_skeleton(&path) {
            Ok(entries) => entries,
            Err(error) => {
                let _ = tx.send(WorkerMessage::TaskFailed(format!(
                    "无法枚举目录 {}: {error}",
                    path.display()
                )));
                return;
            }
        };

        let total = entries.len();
        let _ = tx.send(WorkerMessage::DirectoryEntriesDiscovered {
            task_id,
            path: path.clone(),
            entries: entries.clone(),
            total,
            from_cache: false,
        });

        let jobs = entries
            .iter()
            .filter(|entry| {
                entry.can_enter
                    && matches!(entry.scan_state, ScanState::Pending | ScanState::Scanning)
            })
            .cloned()
            .collect::<Vec<_>>();
        let total_jobs = jobs.len();
        let worker_count = recommended_worker_count(jobs.len());

        if !jobs.is_empty() {
            let chunk_size = jobs.len().div_ceil(worker_count.max(1));
            let (result_tx, result_rx) = mpsc::channel::<DirectoryEntryInfo>();

            for chunk in jobs.chunks(chunk_size.max(1)) {
                let result_tx = result_tx.clone();
                let chunk_entries = chunk.to_vec();
                thread::spawn(move || {
                    for mut entry in chunk_entries {
                        entry.scan_state = ScanState::Scanning;
                        match compute_path_size(&entry.path) {
                            Ok(size) => {
                                entry.size_bytes = size;
                                entry.scan_state = ScanState::Ready;
                            }
                            Err(error) => {
                                entry.size_bytes = 0;
                                entry.scan_state = ScanState::Error;
                                entry.message = Some(error.to_string());
                            }
                        }
                        let _ = result_tx.send(entry);
                    }
                });
            }
            drop(result_tx);

            let mut completed = 0_usize;
            let mut pending_updates = Vec::new();
            let mut last_flush = Instant::now();
            for updated in result_rx {
                completed += 1;
                upsert_directory_entry(&mut entries, updated.clone());
                pending_updates.push(updated);
                let should_flush = pending_updates.len() >= DIRECTORY_UPDATE_BATCH_SIZE
                    || last_flush.elapsed() >= DIRECTORY_UPDATE_THROTTLE
                    || completed == total_jobs;
                if should_flush {
                    let _ = tx.send(WorkerMessage::DirectoryEntriesUpdated {
                        task_id,
                        path: path.clone(),
                        entries: pending_updates.clone(),
                        completed,
                        total: total_jobs,
                        from_cache: false,
                    });
                    pending_updates.clear();
                    last_flush = Instant::now();
                }
            }
        }

        entries.sort_by(|left, right| {
            right
                .size_bytes
                .cmp(&left.size_bytes)
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
        });
        let snapshot = CacheSnapshot {
            path: path.clone(),
            captured_at: chrono::Utc::now(),
            entries: entries.clone(),
        };
        let _ = cache.save_snapshot(&snapshot);
        let _ = tx.send(WorkerMessage::DirectoryScanFinished {
            task_id,
            path,
            entries,
            from_cache: false,
        });
    });
}

fn spawn_cleanup(tx: Sender<WorkerMessage>, items: Vec<CacheDiscovery>) {
    thread::spawn(move || {
        let runner = SystemCommandRunner;
        let selected = items
            .iter()
            .filter(|item| item.selected)
            .collect::<Vec<_>>();
        let total_paths = selected
            .iter()
            .map(|item| item.cleanup_path_count())
            .sum::<usize>();
        let total_bytes = if selected
            .iter()
            .all(|item| item.has_precise_progress_estimates())
        {
            Some(
                selected
                    .iter()
                    .map(|item| item.cleanup_estimated_bytes())
                    .sum(),
            )
        } else {
            None
        };
        let _ = tx.send(WorkerMessage::CleanupStarted {
            total_paths,
            total_bytes,
        });
        let progress_tx = tx.clone();
        let results = execute_cleanup_with_progress(&items, &runner, move |progress| {
            let _ = progress_tx.send(WorkerMessage::CleanupProgress(progress));
        });
        let _ = tx.send(WorkerMessage::CleanupFinished(results));
    });
}

fn progress_status(
    title: &str,
    message: String,
    current: usize,
    total: usize,
) -> BackgroundTaskStatus {
    let mut status = BackgroundTaskStatus::new(title, message, true);
    status.progress_current = current;
    status.progress_total = total;
    status.determinate = total > 0;
    status
}

fn cleanup_status(
    message: String,
    completed_paths: usize,
    total_paths: usize,
    completed_bytes: u64,
    total_bytes: Option<u64>,
) -> BackgroundTaskStatus {
    let mut status = BackgroundTaskStatus::new("缓存清理", message, true);
    status.progress_current = completed_paths;
    status.progress_total = total_paths;
    status.bytes_current = total_bytes.map(|_| completed_bytes);
    status.bytes_total = total_bytes;
    status.determinate = total_bytes.is_some() && total_paths > 0;
    status.progress_label = Some(if let Some(total_bytes) = total_bytes {
        format!(
            "已释放 {} / {}",
            sysclean::ui::format_size(completed_bytes),
            sysclean::ui::format_size(total_bytes)
        )
    } else if total_paths > 0 {
        format!("已处理 {completed_paths}/{total_paths} 个目标")
    } else {
        "正在准备清理目标".into()
    });
    status
}

fn upsert_directory_entry(entries: &mut Vec<DirectoryEntryInfo>, updated: DirectoryEntryInfo) {
    if let Some(existing) = entries.iter_mut().find(|entry| entry.path == updated.path) {
        *existing = updated;
    } else {
        entries.push(updated);
    }
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
