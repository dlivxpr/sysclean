use std::env;
use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::cache_cleaner::{CacheSizeState, CacheTargetKind};
use crate::models::ScanState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    En,
    ZhCn,
}

impl Language {
    pub fn from_config_value(value: &str) -> Self {
        match value.trim() {
            "zh-CN" => Self::ZhCn,
            "en" => Self::En,
            _ => Self::En,
        }
    }

    pub fn help_hint(self) -> &'static str {
        match self {
            Self::En => "Press ? for help",
            Self::ZhCn => "按 ? 查看帮助",
        }
    }

    pub fn cache_cleanup_tab(self) -> &'static str {
        match self {
            Self::En => "Cache Cleanup",
            Self::ZhCn => "缓存清理",
        }
    }

    pub fn space_explorer_tab(self) -> &'static str {
        match self {
            Self::En => "Space Explorer",
            Self::ZhCn => "目录分析",
        }
    }

    pub fn status_title(self) -> &'static str {
        match self {
            Self::En => " Status ",
            Self::ZhCn => " 状态 ",
        }
    }

    pub fn cache_cleanup_title(self) -> &'static str {
        match self {
            Self::En => " Cache Cleanup ",
            Self::ZhCn => " 缓存清理 ",
        }
    }

    pub fn cache_details_title(self) -> &'static str {
        match self {
            Self::En => " Details ",
            Self::ZhCn => " 明细 ",
        }
    }

    pub fn path_title(self) -> &'static str {
        match self {
            Self::En => " Path ",
            Self::ZhCn => " 路径 ",
        }
    }

    pub fn space_explorer_title(self) -> &'static str {
        match self {
            Self::En => " Space Explorer ",
            Self::ZhCn => " 目录分析 ",
        }
    }

    pub fn detail_title(self) -> &'static str {
        match self {
            Self::En => " Detail ",
            Self::ZhCn => " 详情 ",
        }
    }

    pub fn shortcuts_title(self) -> &'static str {
        match self {
            Self::En => " Shortcuts ",
            Self::ZhCn => " 常用快捷键 ",
        }
    }

    pub fn help_title(self) -> &'static str {
        match self {
            Self::En => " Help ",
            Self::ZhCn => " 帮助 ",
        }
    }

    pub fn delete_confirmation_title(self) -> &'static str {
        match self {
            Self::En => " Delete Confirmation ",
            Self::ZhCn => " 删除确认 ",
        }
    }

    pub fn cleanup_results_title(self) -> &'static str {
        match self {
            Self::En => " Cleanup Results ",
            Self::ZhCn => " 清理结果 ",
        }
    }

    pub fn cache_column_cache(self) -> &'static str {
        match self {
            Self::En => "Cache",
            Self::ZhCn => "缓存",
        }
    }

    pub fn cache_column_reclaimable(self) -> &'static str {
        match self {
            Self::En => "Reclaimable",
            Self::ZhCn => "可释放",
        }
    }

    pub fn cache_column_used(self) -> &'static str {
        match self {
            Self::En => "Used",
            Self::ZhCn => "占用",
        }
    }

    pub fn cache_column_status(self) -> &'static str {
        match self {
            Self::En => "Status",
            Self::ZhCn => "状态",
        }
    }

    pub fn directory_column_name(self) -> &'static str {
        match self {
            Self::En => "Directory",
            Self::ZhCn => "目录",
        }
    }

    pub fn directory_column_size(self) -> &'static str {
        match self {
            Self::En => "Size",
            Self::ZhCn => "大小",
        }
    }

    pub fn directory_column_status(self) -> &'static str {
        self.cache_column_status()
    }

    pub fn detail_name_label(self) -> &'static str {
        match self {
            Self::En => "Name",
            Self::ZhCn => "名称",
        }
    }

    pub fn detail_description_label(self) -> &'static str {
        match self {
            Self::En => "Description",
            Self::ZhCn => "说明",
        }
    }

    pub fn detail_status_label(self) -> &'static str {
        match self {
            Self::En => "Status",
            Self::ZhCn => "状态",
        }
    }

    pub fn detail_path_count_label(self) -> &'static str {
        match self {
            Self::En => "Path count",
            Self::ZhCn => "路径数",
        }
    }

    pub fn detail_reclaimable_label(self) -> &'static str {
        match self {
            Self::En => "Reclaimable",
            Self::ZhCn => "可释放",
        }
    }

    pub fn detail_note_label(self) -> &'static str {
        match self {
            Self::En => "Note",
            Self::ZhCn => "备注",
        }
    }

    pub fn detail_paths_label(self) -> &'static str {
        match self {
            Self::En => "Paths",
            Self::ZhCn => "路径",
        }
    }

    pub fn current_path_label(self) -> &'static str {
        match self {
            Self::En => "Current path",
            Self::ZhCn => "当前位置",
        }
    }

    pub fn filter_label(self) -> &'static str {
        match self {
            Self::En => "Filter",
            Self::ZhCn => "筛选",
        }
    }

    pub fn inputting_suffix(self) -> &'static str {
        match self {
            Self::En => "  (typing)",
            Self::ZhCn => "  (输入中)",
        }
    }

    pub fn not_loaded(self) -> &'static str {
        match self {
            Self::En => "<not loaded>",
            Self::ZhCn => "<未加载>",
        }
    }

    pub fn none_text(self) -> &'static str {
        match self {
            Self::En => "None",
            Self::ZhCn => "无",
        }
    }

    pub fn yes_text(self) -> &'static str {
        match self {
            Self::En => "Yes",
            Self::ZhCn => "是",
        }
    }

    pub fn no_text(self) -> &'static str {
        match self {
            Self::En => "No",
            Self::ZhCn => "否",
        }
    }

    pub fn no_cache_items(self) -> &'static str {
        match self {
            Self::En => "No cache targets available.",
            Self::ZhCn => "暂无缓存项。",
        }
    }

    pub fn no_directory_entries(self) -> &'static str {
        match self {
            Self::En => "No directories to display.",
            Self::ZhCn => "没有可显示的目录。",
        }
    }

    pub fn docker_cli_only_note(self) -> &'static str {
        match self {
            Self::En => {
                "This target is cleaned through the docker CLI and does not expose arbitrary file deletion."
            }
            Self::ZhCn => "此目标通过 docker CLI 清理，不直接暴露删除任意路径。",
        }
    }

    pub fn directory_label(self) -> &'static str {
        match self {
            Self::En => "Directory",
            Self::ZhCn => "目录",
        }
    }

    pub fn path_label(self) -> &'static str {
        match self {
            Self::En => "Path",
            Self::ZhCn => "路径",
        }
    }

    pub fn size_label(self) -> &'static str {
        match self {
            Self::En => "Size",
            Self::ZhCn => "大小",
        }
    }

    pub fn can_enter_label(self) -> &'static str {
        match self {
            Self::En => "Enterable",
            Self::ZhCn => "可进入",
        }
    }

    pub fn remark_label(self) -> &'static str {
        match self {
            Self::En => "Remark",
            Self::ZhCn => "备注",
        }
    }

    pub fn filtering_shortcuts(self) -> &'static str {
        match self {
            Self::En => "Type filter   Enter apply   Esc cancel   Backspace delete",
            Self::ZhCn => "输入关键字   Enter 应用   Esc 取消   Backspace 删除",
        }
    }

    pub fn cache_shortcuts(self) -> &'static str {
        match self {
            Self::En => {
                "↑↓ Select   Space Toggle   a Select all   r Rescan   d Delete   ? Help   q Quit"
            }
            Self::ZhCn => "↑↓ 选择   Space 勾选   a 全选   r 重扫   d 删除   ? 帮助   q 退出",
        }
    }

    pub fn explorer_shortcuts(self) -> &'static str {
        match self {
            Self::En => {
                "↑↓ Select   Enter Open   Backspace Up   / Filter   o Open dir   r Rescan   ? Help"
            }
            Self::ZhCn => {
                "↑↓ 选择   Enter 进入   Backspace 返回   / 过滤   o 打开目录   r 重扫   ? 帮助"
            }
        }
    }

    pub fn in_progress(self) -> &'static str {
        match self {
            Self::En => "In progress",
            Self::ZhCn => "进行中",
        }
    }

    pub fn background_task(self) -> &'static str {
        match self {
            Self::En => "Background task",
            Self::ZhCn => "后台任务",
        }
    }

    pub fn help_dialog_text(self) -> &'static str {
        match self {
            Self::En => {
                "Global shortcuts\nTab / ←→ Switch workspace\nq Quit\n? Open help\nEsc Close dialog or cancel input\n\nCache Cleanup\n↑↓ Select cache target\nSpace Toggle selection\nA Select all or clear all\nR Rescan caches\nD Open delete confirmation\nPaths appear first and sizes update in the background\n\nSpace Explorer\n↑↓ Select directory\nEnter Enter directory\nBackspace Go to parent\nHome / End Jump to start/end\nPgUp / PgDn Page faster\n/ Start filtering\nO Open in Explorer\nAfter entering a directory, the skeleton renders first and sizes reorder progressively"
            }
            Self::ZhCn => {
                "全局快捷键\nTab / ←→ 切换工作区\nq 退出\n? 打开帮助\nEsc 关闭弹窗或取消输入\n\n缓存清理\n↑↓ 选择缓存\nSpace 勾选\nA 全选或反选\nR 重扫缓存\nD 打开删除确认\n路径会先显示，大小会后台逐项更新\n\n目录分析\n↑↓ 选择目录\nEnter 进入目录\nBackspace 返回上级\nHome / End 跳到首尾\nPgUp / PgDn 快速翻页\n/ 进入过滤\nO 打开资源管理器\n进入目录后会先展示骨架，再边扫边重排"
            }
        }
    }

    pub fn delete_confirmation_body(self, items: &str, total: &str) -> String {
        match self {
            Self::En => format!(
                "The following cache targets will be deleted:\n{}\n\nEstimated reclaimable: {}\n\nPress Enter to confirm, Esc to cancel.",
                items, total
            ),
            Self::ZhCn => format!(
                "即将删除以下缓存项:\n{}\n\n预计释放: {}\n\n按 Enter 确认, Esc 取消。",
                items, total
            ),
        }
    }

    pub fn no_selected_cache_items(self) -> &'static str {
        match self {
            Self::En => "No cache targets selected.",
            Self::ZhCn => "没有选中的缓存项。",
        }
    }

    pub fn cleanup_result_line(self, label: &str, size: &str) -> String {
        match self {
            Self::En => format!("• {label}: reclaimed {size}"),
            Self::ZhCn => format!("• {label}: 释放 {size}"),
        }
    }

    pub fn cleanup_result_line_with_skipped(
        self,
        label: &str,
        size: &str,
        skipped: usize,
    ) -> String {
        match self {
            Self::En => format!("• {label}: reclaimed {size} | skipped {skipped} item(s)"),
            Self::ZhCn => format!("• {label}: 释放 {size} | 跳过 {skipped} 项"),
        }
    }

    pub fn cleanup_summary_body(self, total: &str, body: &str) -> String {
        match self {
            Self::En => format!(
                "Cleanup finished.\nTotal reclaimed: {}\n\n{}\n\nPress Esc or Enter to close.",
                total, body
            ),
            Self::ZhCn => format!(
                "本次清理完成。\n总释放: {}\n\n{}\n\n按 Esc 或 Enter 关闭。",
                total, body
            ),
        }
    }

    pub fn cache_size_state(self, state: CacheSizeState) -> &'static str {
        match state {
            CacheSizeState::Pending => match self {
                Self::En => "Pending",
                Self::ZhCn => "待计算",
            },
            CacheSizeState::Scanning => match self {
                Self::En => "Scanning",
                Self::ZhCn => "计算中",
            },
            CacheSizeState::Ready => "",
            CacheSizeState::Unavailable => match self {
                Self::En => "Unavailable",
                Self::ZhCn => "不可用",
            },
            CacheSizeState::Error => match self {
                Self::En => "Failed",
                Self::ZhCn => "失败",
            },
        }
    }

    pub fn scan_state(self, state: ScanState) -> &'static str {
        match state {
            ScanState::Ready => match self {
                Self::En => "Ready",
                Self::ZhCn => "就绪",
            },
            ScanState::Cached => match self {
                Self::En => "Cached",
                Self::ZhCn => "缓存",
            },
            ScanState::Scanning => match self {
                Self::En => "Scanning",
                Self::ZhCn => "扫描中",
            },
            ScanState::Pending => match self {
                Self::En => "Pending",
                Self::ZhCn => "待扫描",
            },
            ScanState::Skipped => match self {
                Self::En => "Skipped",
                Self::ZhCn => "跳过",
            },
            ScanState::Error => match self {
                Self::En => "Failed",
                Self::ZhCn => "失败",
            },
        }
    }

    pub fn cache_status_command_target(self) -> &'static str {
        match self {
            Self::En => "Command target",
            Self::ZhCn => "命令型目标",
        }
    }

    pub fn cache_status_checked_zero(self) -> &'static str {
        match self {
            Self::En => "Checked 0B",
            Self::ZhCn => "已检查 0B",
        }
    }

    pub fn cache_status_reclaimable(self) -> &'static str {
        match self {
            Self::En => "Reclaimable",
            Self::ZhCn => "可清理",
        }
    }

    pub fn startup_title(self) -> &'static str {
        match self {
            Self::En => "Startup",
            Self::ZhCn => "启动",
        }
    }

    pub fn startup_message(self) -> &'static str {
        match self {
            Self::En => "Preparing cache and directory scans",
            Self::ZhCn => "正在准备缓存与目录扫描",
        }
    }

    pub fn cache_scan_title(self) -> &'static str {
        match self {
            Self::En => "Cache scan",
            Self::ZhCn => "缓存扫描",
        }
    }

    pub fn cache_discovered_message(self, discovered: usize, total: usize) -> String {
        match self {
            Self::En => format!("Discovered {discovered}/{total} cache targets and sizing them"),
            Self::ZhCn => format!("已发现 {discovered}/{total} 个缓存目标，正在计算大小"),
        }
    }

    pub fn cache_discovered_progress(self, discovered: usize, total: usize) -> String {
        match self {
            Self::En => format!("Discovered {discovered}/{total}"),
            Self::ZhCn => format!("已发现 {discovered}/{total}"),
        }
    }

    pub fn cache_sizing_message(self, completed: usize, total: usize) -> String {
        match self {
            Self::En => format!("Discovered {total}/{total} targets, sized {completed}/{total}"),
            Self::ZhCn => {
                format!("已发现 {total}/{total} 个缓存目标，正在计算 {completed}/{total} 个大小")
            }
        }
    }

    pub fn progress_completed(self, completed: usize, total: usize) -> String {
        match self {
            Self::En => format!("Completed {completed}/{total}"),
            Self::ZhCn => format!("已完成 {completed}/{total}"),
        }
    }

    pub fn no_cache_targets(self) -> &'static str {
        match self {
            Self::En => "No cache targets available",
            Self::ZhCn => "未发现可扫描的缓存目标",
        }
    }

    pub fn cache_scan_finished(self) -> &'static str {
        match self {
            Self::En => "All cache paths are listed and size calculation finished",
            Self::ZhCn => "缓存路径已全部展示，大小计算完成",
        }
    }

    pub fn directory_scan_title(self) -> &'static str {
        match self {
            Self::En => "Directory scan",
            Self::ZhCn => "目录扫描",
        }
    }

    pub fn enumerating_directories(self, path: &Path) -> String {
        match self {
            Self::En => format!("Enumerating direct subdirectories of {}", path.display()),
            Self::ZhCn => format!("正在枚举 {} 的直接子目录", path.display()),
        }
    }

    pub fn directory_loaded_from_cache(self, total: usize) -> String {
        match self {
            Self::En => format!("Loaded {total} subdirectories from cache"),
            Self::ZhCn => format!("已从缓存加载 {total} 个子目录"),
        }
    }

    pub fn directory_discovered_and_sizing(self, total: usize) -> String {
        match self {
            Self::En => format!("Enumerated {total} subdirectories, sizing them now"),
            Self::ZhCn => format!("已枚举 {total} 个子目录，正在逐项计算大小"),
        }
    }

    pub fn cache_hit_items(self, total: usize) -> String {
        match self {
            Self::En => format!("Cache hit {total} item(s)"),
            Self::ZhCn => format!("缓存命中 {total} 项"),
        }
    }

    pub fn cached_directory_progress(self, completed: usize, total: usize) -> String {
        match self {
            Self::En => format!("Loaded cached results {completed}/{total}"),
            Self::ZhCn => format!("缓存结果已加载 {completed}/{total}"),
        }
    }

    pub fn directory_sizing_progress(self, completed: usize, total: usize) -> String {
        match self {
            Self::En => format!("Enumerated {total} subdirectories, sizing {completed}/{total}"),
            Self::ZhCn => format!("已枚举 {total} 个子目录，正在计算 {completed}/{total} 个大小"),
        }
    }

    pub fn directory_cache_loaded_done(self) -> &'static str {
        match self {
            Self::En => "Directory skeleton loaded from cache, press r to force a rescan",
            Self::ZhCn => "目录骨架已从缓存加载，按 r 可强制重扫",
        }
    }

    pub fn directory_progress_finished(self) -> &'static str {
        match self {
            Self::En => "Directory skeleton is visible and progressive sizing finished",
            Self::ZhCn => "目录骨架已展示，大小已渐进更新完成",
        }
    }

    pub fn cleanup_finished(self) -> &'static str {
        match self {
            Self::En => "Cache cleanup finished",
            Self::ZhCn => "缓存清理已完成",
        }
    }

    pub fn preparing_cache_cleanup(self) -> &'static str {
        match self {
            Self::En => "Preparing cache cleanup",
            Self::ZhCn => "正在准备删除缓存",
        }
    }

    pub fn cleaning_target(self, label: &str, completed: usize, total: usize) -> String {
        match self {
            Self::En => format!("Cleaning {label} ({completed}/{total})"),
            Self::ZhCn => format!("正在清理 {label} ({completed}/{total})"),
        }
    }

    pub fn cleanup_title(self) -> &'static str {
        match self {
            Self::En => "Cache cleanup",
            Self::ZhCn => "缓存清理",
        }
    }

    pub fn deleting_ready_cache_items(self) -> &'static str {
        match self {
            Self::En => "Deleting cache targets that already have size information",
            Self::ZhCn => "正在删除已完成大小计算的缓存项",
        }
    }

    pub fn task_display_cancelled(self) -> &'static str {
        match self {
            Self::En => "Current task display hidden; stale results will be ignored",
            Self::ZhCn => "已取消当前任务显示，旧结果会被忽略",
        }
    }

    pub fn rescanning_cache_paths(self) -> &'static str {
        match self {
            Self::En => "Rediscovering cache paths",
            Self::ZhCn => "正在重新发现缓存路径",
        }
    }

    pub fn filter_prompt(self) -> &'static str {
        match self {
            Self::En => "Type a filter keyword and press Enter to apply",
            Self::ZhCn => "输入筛选关键字，按 Enter 应用",
        }
    }

    pub fn opened_in_explorer(self, path: &Path) -> String {
        match self {
            Self::En => format!("Opened {} in File Explorer", path.display()),
            Self::ZhCn => format!("已在资源管理器中打开 {}", path.display()),
        }
    }

    pub fn entered_directory(self, path: &Path) -> String {
        match self {
            Self::En => format!("Entered {}, enumerating subdirectories", path.display()),
            Self::ZhCn => format!("已进入 {}，正在枚举子目录", path.display()),
        }
    }

    pub fn exited_filter_mode(self) -> &'static str {
        match self {
            Self::En => "Exited filter mode",
            Self::ZhCn => "已退出过滤模式",
        }
    }

    pub fn cleared_filter(self) -> &'static str {
        match self {
            Self::En => "Filter cleared",
            Self::ZhCn => "已清空过滤条件",
        }
    }

    pub fn applied_filter(self, filter: &str) -> String {
        match self {
            Self::En => format!("Filter: {filter}"),
            Self::ZhCn => format!("过滤: {filter}"),
        }
    }

    pub fn cache_scan_failed(self, error: &str) -> String {
        match self {
            Self::En => format!("Cache scan failed: {error}"),
            Self::ZhCn => format!("缓存扫描失败: {error}"),
        }
    }

    pub fn init_scan_cache_failed(self, error: &str) -> String {
        match self {
            Self::En => format!("Failed to initialize scan cache: {error}"),
            Self::ZhCn => format!("初始化扫描缓存失败: {error}"),
        }
    }

    pub fn directory_scan_failed(self, error: &str) -> String {
        match self {
            Self::En => format!("Directory scan failed: {error}"),
            Self::ZhCn => format!("目录扫描失败: {error}"),
        }
    }

    pub fn unable_to_enumerate_directory(self, path: &Path, error: &str) -> String {
        match self {
            Self::En => format!("Unable to enumerate directory {}: {error}", path.display()),
            Self::ZhCn => format!("无法枚举目录 {}: {error}", path.display()),
        }
    }

    pub fn cleanup_progress_bytes(self, current: &str, total: &str) -> String {
        match self {
            Self::En => format!("Reclaimed {current} / {total}"),
            Self::ZhCn => format!("已释放 {current} / {total}"),
        }
    }

    pub fn cleanup_progress_targets(self, completed: usize, total: usize) -> String {
        match self {
            Self::En => format!("Processed {completed}/{total} target(s)"),
            Self::ZhCn => format!("已处理 {completed}/{total} 个目标"),
        }
    }

    pub fn cleanup_progress_preparing(self) -> &'static str {
        match self {
            Self::En => "Preparing cleanup targets",
            Self::ZhCn => "正在准备清理目标",
        }
    }

    pub fn wait_for_selected_cache_sizes(self, waiting_items: &str) -> String {
        match self {
            Self::En => format!(
                "Wait for selected cache targets to finish size calculation: {waiting_items}"
            ),
            Self::ZhCn => format!("请等待所选缓存大小计算完成: {waiting_items}"),
        }
    }

    pub fn select_at_least_one_cache(self) -> &'static str {
        match self {
            Self::En => "Select at least one cache target first",
            Self::ZhCn => "请先选择至少一个缓存项",
        }
    }

    pub fn cache_description(self, kind: CacheTargetKind) -> &'static str {
        match kind {
            CacheTargetKind::Uv => match self {
                Self::En => "Global cache for the uv Python package manager",
                Self::ZhCn => "Python 包管理器 uv 的全局缓存",
            },
            CacheTargetKind::Npm => match self {
                Self::En => "npm cache and downloaded artifacts",
                Self::ZhCn => "npm cache 与下载产物",
            },
            CacheTargetKind::Pnpm => match self {
                Self::En => "Global pnpm store cache",
                Self::ZhCn => "pnpm store 全局缓存",
            },
            CacheTargetKind::Docker => match self {
                Self::En => "Docker builder cache and dangling build cache",
                Self::ZhCn => "Docker builder cache 与 dangling build cache",
            },
            CacheTargetKind::Cargo => match self {
                Self::En => "Cargo registry and git cache",
                Self::ZhCn => "Cargo registry 与 git 缓存",
            },
        }
    }

    pub fn docker_builder_cache_note(self) -> &'static str {
        match self {
            Self::En => "Only docker builder cache is cleaned",
            Self::ZhCn => "仅清理 docker builder cache",
        }
    }

    pub fn docker_unavailable_note(self) -> &'static str {
        match self {
            Self::En => "docker CLI was not detected or is unavailable in this session",
            Self::ZhCn => "未检测到 docker CLI 或当前会话不可用",
        }
    }

    pub fn cache_paths_not_found(self) -> &'static str {
        match self {
            Self::En => "No cache paths found",
            Self::ZhCn => "未发现缓存路径",
        }
    }

    pub fn path_size_failed(self, path: &Path, error: &str) -> String {
        match self {
            Self::En => format!("{} size calculation failed: {error}", path.display()),
            Self::ZhCn => format!("{} 统计失败: {error}", path.display()),
        }
    }

    pub fn path_missing(self, path: &Path) -> String {
        match self {
            Self::En => format!("{} does not exist", path.display()),
            Self::ZhCn => format!("{} 不存在", path.display()),
        }
    }

    pub fn path_delete_failed(self, path: &Path, error: &str) -> String {
        match self {
            Self::En => format!("{} delete failed: {error}", path.display()),
            Self::ZhCn => format!("{} 删除失败: {error}", path.display()),
        }
    }

    pub fn unknown_entry_name(self) -> &'static str {
        match self {
            Self::En => "<unknown>",
            Self::ZhCn => "<未知>",
        }
    }

    pub fn skipped_symlink(self) -> &'static str {
        match self {
            Self::En => "Skipped symbolic link or junction",
            Self::ZhCn => "符号链接或 junction 已跳过",
        }
    }

    pub fn home_dir_not_found(self) -> &'static str {
        match self {
            Self::En => "Unable to locate the current user's home directory",
            Self::ZhCn => "无法定位当前用户目录",
        }
    }

    pub fn local_app_data_not_found(self) -> &'static str {
        match self {
            Self::En => "Unable to locate the LocalAppData directory",
            Self::ZhCn => "无法定位 LocalAppData 目录",
        }
    }

    pub fn app_cache_dir_not_found(self) -> &'static str {
        match self {
            Self::En => "Unable to locate the application cache directory",
            Self::ZhCn => "无法定位应用缓存目录",
        }
    }

    pub fn open_in_explorer_failed(self, path: &Path) -> String {
        match self {
            Self::En => format!("Unable to open File Explorer for {}", path.display()),
            Self::ZhCn => format!("无法打开资源管理器: {}", path.display()),
        }
    }
}

pub fn load_language_from_file(path: &Path) -> Result<Language> {
    if !path.exists() {
        return Ok(Language::En);
    }

    let payload = fs::read_to_string(path)?;
    let mut in_ui_section = false;
    for raw_line in payload.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_ui_section = &line[1..line.len() - 1] == "ui";
            continue;
        }
        if !in_ui_section {
            continue;
        }
        if let Some(value) = line.strip_prefix("language=") {
            return Ok(Language::from_config_value(value));
        }
    }

    Ok(Language::En)
}

pub fn load_installed_language() -> Language {
    let ini_path = env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|parent| parent.join("sysclean.ini")));

    ini_path
        .as_deref()
        .and_then(|path| load_language_from_file(path).ok())
        .unwrap_or(Language::En)
}
