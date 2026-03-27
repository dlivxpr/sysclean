use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::i18n::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheTargetKind {
    Uv,
    Npm,
    Pnpm,
    Docker,
    Cargo,
}

pub const SUPPORTED_CACHE_TARGETS: [CacheTargetKind; 5] = [
    CacheTargetKind::Uv,
    CacheTargetKind::Npm,
    CacheTargetKind::Pnpm,
    CacheTargetKind::Docker,
    CacheTargetKind::Cargo,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheSizeState {
    Pending,
    Scanning,
    Ready,
    Unavailable,
    Error,
}

impl CacheTargetKind {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Uv => "uv",
            Self::Npm => "npm",
            Self::Pnpm => "pnpm",
            Self::Docker => "docker",
            Self::Cargo => "cargo",
        }
    }

    /// Returns the official CLI cleanup command for this tool, if available.
    /// Tools with `Some` use CLI-based cleanup; `None` falls back to manual file deletion.
    pub fn cleanup_command(self) -> Option<(&'static str, &'static [&'static str])> {
        match self {
            Self::Uv => Some(("uv", &["cache", "clean"])),
            Self::Npm => Some(("npm", &["cache", "clean", "--force"])),
            Self::Pnpm => Some(("pnpm", &["store", "prune"])),
            Self::Docker => Some(("docker", &["builder", "prune", "-a", "-f"])),
            Self::Cargo => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub stdout: String,
}

impl CommandOutput {
    pub fn success(stdout: impl Into<String>) -> Self {
        Self {
            stdout: stdout.into(),
        }
    }
}

pub trait CommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<CommandOutput>;
}

#[derive(Debug, Clone, Default)]
pub struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<CommandOutput> {
        #[cfg(target_os = "windows")]
        let output = Command::new("cmd")
            .arg("/C")
            .arg(program)
            .args(args.iter().map(OsStr::new))
            .output()
            .with_context(|| format!("failed to run {program}"))?;
        #[cfg(not(target_os = "windows"))]
        let output = Command::new(program)
            .args(args.iter().map(OsStr::new))
            .output()
            .with_context(|| format!("failed to run {program}"))?;
        if !output.status.success() {
            return Err(anyhow!("{program} exited with {}", output.status));
        }
        Ok(CommandOutput::success(
            String::from_utf8_lossy(&output.stdout).trim().to_string(),
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachePathEstimate {
    pub path: PathBuf,
    pub estimated_bytes: Option<u64>,
}

impl CachePathEstimate {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            estimated_bytes: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheDiscovery {
    pub kind: CacheTargetKind,
    pub label: String,
    pub description: String,
    pub paths: Vec<PathBuf>,
    pub path_estimates: Vec<CachePathEstimate>,
    pub available: bool,
    pub size_state: CacheSizeState,
    pub reclaimable_bytes: Option<u64>,
    pub total_bytes: u64,
    pub selected: bool,
    pub note: Option<String>,
}

impl CacheDiscovery {
    pub fn new(kind: CacheTargetKind, label: String, paths: Vec<PathBuf>) -> Self {
        Self {
            kind,
            label,
            description: Language::En.cache_description(kind).to_string(),
            path_estimates: paths.iter().cloned().map(CachePathEstimate::new).collect(),
            paths,
            available: true,
            size_state: CacheSizeState::Pending,
            reclaimable_bytes: None,
            total_bytes: 0,
            selected: false,
            note: None,
        }
    }

    fn sync_path_estimates(&mut self) {
        self.path_estimates = self
            .paths
            .iter()
            .cloned()
            .map(CachePathEstimate::new)
            .collect();
    }

    pub fn cleanup_path_count(&self) -> usize {
        if self.kind.cleanup_command().is_some() {
            1
        } else {
            self.path_estimates.len().max(1)
        }
    }

    pub fn cleanup_estimated_bytes(&self) -> u64 {
        if self.path_estimates.is_empty() {
            self.reclaimable_bytes.unwrap_or(self.total_bytes)
        } else {
            self.path_estimates
                .iter()
                .map(|estimate| estimate.estimated_bytes.unwrap_or_default())
                .sum()
        }
    }

    pub fn has_precise_progress_estimates(&self) -> bool {
        !self.path_estimates.is_empty()
            && self
                .path_estimates
                .iter()
                .all(|estimate| estimate.estimated_bytes.is_some())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupPreview {
    pub items: Vec<CacheDiscovery>,
    pub total_reclaimable_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupOutcome {
    pub label: String,
    pub bytes_reclaimed: u64,
    pub skipped: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupProgress {
    pub current_label: String,
    pub completed_bytes: u64,
    pub total_bytes: Option<u64>,
    pub completed_paths: usize,
    pub total_paths: usize,
}

#[derive(Debug, Clone, Copy)]
struct CleanupExecutionState {
    total_paths: usize,
    total_bytes: Option<u64>,
    completed_paths: usize,
    completed_bytes: u64,
}

pub fn discover_cache_target(
    kind: CacheTargetKind,
    language: Language,
    runner: &impl CommandRunner,
    user_profile: Option<PathBuf>,
    local_app_data: Option<PathBuf>,
) -> Result<CacheDiscovery> {
    let mut discovery =
        discover_cache_metadata(kind, language, runner, user_profile, local_app_data)?;
    if kind != CacheTargetKind::Docker {
        populate_cache_size(&mut discovery, language)?;
    }
    Ok(discovery)
}

pub fn discover_cache_metadata(
    kind: CacheTargetKind,
    language: Language,
    runner: &impl CommandRunner,
    user_profile: Option<PathBuf>,
    local_app_data: Option<PathBuf>,
) -> Result<CacheDiscovery> {
    let mut discovery = CacheDiscovery::new(kind, kind.display_name().to_string(), Vec::new());
    discovery.description = language.cache_description(kind).to_string();

    match kind {
        CacheTargetKind::Uv => {
            discovery.paths = command_path(runner, "uv", &["cache", "dir"])
                .into_iter()
                .collect::<Vec<_>>();
            if discovery.paths.is_empty()
                && let Some(base) = local_app_data
            {
                discovery.paths.push(base.join("uv").join("cache"));
            }
        }
        CacheTargetKind::Npm => {
            discovery.paths = command_path(runner, "npm", &["config", "get", "cache"])
                .into_iter()
                .collect::<Vec<_>>();
            if discovery.paths.is_empty()
                && let Some(base) = local_app_data
            {
                discovery.paths.push(base.join("npm-cache"));
            }
        }
        CacheTargetKind::Pnpm => {
            discovery.paths = command_path(runner, "pnpm", &["store", "path"])
                .into_iter()
                .collect::<Vec<_>>();
            if discovery.paths.is_empty() {
                // 命令失败时，扫描 pnpm/store/ 目录下实际存在的版本子目录
                let store_bases: Vec<PathBuf> = [
                    local_app_data
                        .as_ref()
                        .map(|b| b.join("pnpm").join("store")),
                    user_profile
                        .as_ref()
                        .map(|h| h.join("AppData").join("Local").join("pnpm").join("store")),
                ]
                .into_iter()
                .flatten()
                .collect();

                for store_base in &store_bases {
                    if let Ok(entries) = fs::read_dir(store_base) {
                        for entry in entries.flatten() {
                            let p = entry.path();
                            if p.is_dir() {
                                discovery.paths.push(p);
                            }
                        }
                    }
                }

                // 若扫描也无结果，保留 v3 作为最后兜底
                if discovery.paths.is_empty() {
                    for store_base in &store_bases {
                        discovery.paths.push(store_base.join("v3"));
                    }
                }
            }
        }
        CacheTargetKind::Cargo => {
            if let Some(home) = user_profile {
                discovery.paths = vec![
                    home.join(".cargo").join("registry"),
                    home.join(".cargo").join("git"),
                ];
            }
        }
        CacheTargetKind::Docker => {
            let output = runner.run("docker", &["system", "df", "--format", "json"]);
            match output {
                Ok(output) => {
                    discovery.available = true;
                    discovery.size_state = CacheSizeState::Ready;
                    discovery.note = Some(language.docker_builder_cache_note().into());
                    discovery.reclaimable_bytes = parse_docker_reclaimable(&output.stdout);
                }
                Err(_) => {
                    discovery.available = false;
                    discovery.size_state = CacheSizeState::Unavailable;
                    discovery.note = Some(language.docker_unavailable_note().into());
                }
            }
        }
    }

    if kind != CacheTargetKind::Docker {
        discovery.paths.sort();
        discovery.paths.dedup();
        discovery.sync_path_estimates();
        discovery.available =
            discovery.paths.iter().any(|path| path.exists()) || !discovery.paths.is_empty();
        if discovery.paths.is_empty() {
            discovery.available = false;
            discovery.size_state = CacheSizeState::Unavailable;
            discovery.note = Some(language.cache_paths_not_found().into());
        } else {
            discovery.size_state = CacheSizeState::Pending;
        }
    }

    Ok(discovery)
}

pub fn populate_cache_size(item: &mut CacheDiscovery, language: Language) -> Result<()> {
    if !item.available {
        item.size_state = CacheSizeState::Unavailable;
        item.reclaimable_bytes = None;
        item.total_bytes = 0;
        item.sync_path_estimates();
        return Ok(());
    }

    if item.kind == CacheTargetKind::Docker {
        item.size_state = if item.reclaimable_bytes.is_some() {
            CacheSizeState::Ready
        } else {
            CacheSizeState::Error
        };
        return Ok(());
    }

    if item.path_estimates.len() != item.paths.len() {
        item.sync_path_estimates();
    }

    item.size_state = CacheSizeState::Scanning;
    let mut total_bytes = 0_u64;
    let mut had_error = false;
    for estimate in &mut item.path_estimates {
        match compute_path_size(&estimate.path) {
            Ok(size) => {
                estimate.estimated_bytes = Some(size);
                total_bytes += size;
            }
            Err(error) => {
                had_error = true;
                estimate.estimated_bytes = None;
                item.note = Some(language.path_size_failed(&estimate.path, &error.to_string()));
            }
        }
    }
    item.total_bytes = total_bytes;
    item.reclaimable_bytes = Some(total_bytes);
    item.size_state = if had_error {
        CacheSizeState::Error
    } else {
        CacheSizeState::Ready
    };
    Ok(())
}

pub fn discover_all_caches(
    language: Language,
    runner: &impl CommandRunner,
    user_profile: PathBuf,
    local_app_data: PathBuf,
) -> Result<Vec<CacheDiscovery>> {
    SUPPORTED_CACHE_TARGETS
        .into_iter()
        .map(|kind| {
            discover_cache_target(
                kind,
                language,
                runner,
                Some(user_profile.clone()),
                Some(local_app_data.clone()),
            )
        })
        .collect::<Result<Vec<_>>>()
        .map(|mut items| {
            for item in &mut items {
                let _ = populate_cache_size(item, language);
            }
            items
        })
}

pub fn build_cleanup_preview(items: &[CacheDiscovery]) -> CleanupPreview {
    let selected: Vec<CacheDiscovery> = items
        .iter()
        .filter(|item| item.selected && item.size_state == CacheSizeState::Ready)
        .cloned()
        .collect();
    let total_reclaimable_bytes = selected
        .iter()
        .map(|item| item.reclaimable_bytes.unwrap_or(item.total_bytes))
        .sum();
    CleanupPreview {
        items: selected,
        total_reclaimable_bytes,
    }
}

pub fn execute_cleanup(
    items: &[CacheDiscovery],
    language: Language,
    runner: &impl CommandRunner,
) -> Vec<CleanupOutcome> {
    execute_cleanup_with_progress(items, language, runner, |_| {})
}

pub fn execute_cleanup_with_progress(
    items: &[CacheDiscovery],
    language: Language,
    runner: &impl CommandRunner,
    mut on_progress: impl FnMut(CleanupProgress),
) -> Vec<CleanupOutcome> {
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
    let mut state = CleanupExecutionState {
        total_paths,
        total_bytes,
        completed_paths: 0,
        completed_bytes: 0,
    };

    selected
        .into_iter()
        .map(|item| execute_single_cleanup(item, language, runner, &mut state, &mut on_progress))
        .collect()
}

fn execute_single_cleanup(
    item: &CacheDiscovery,
    language: Language,
    runner: &impl CommandRunner,
    state: &mut CleanupExecutionState,
    on_progress: &mut impl FnMut(CleanupProgress),
) -> CleanupOutcome {
    // CLI-based cleanup: uv, npm, pnpm, docker all use their official cleanup commands
    if let Some((program, args)) = item.kind.cleanup_command() {
        let estimated = item.cleanup_estimated_bytes();
        let result = runner.run(program, args);
        state.completed_paths += 1;
        state.completed_bytes += estimated;
        on_progress(CleanupProgress {
            current_label: item.label.clone(),
            completed_bytes: state.completed_bytes,
            total_bytes: state.total_bytes,
            completed_paths: state.completed_paths,
            total_paths: state.total_paths,
        });
        return CleanupOutcome {
            label: item.label.clone(),
            bytes_reclaimed: if result.is_ok() { estimated } else { 0 },
            skipped: result
                .err()
                .map(|err| vec![err.to_string()])
                .unwrap_or_default(),
        };
    }

    // Manual file deletion fallback (cargo)
    let mut reclaimed = 0;
    let mut skipped = Vec::new();
    for estimate in &item.path_estimates {
        if !estimate.path.exists() {
            skipped.push(language.path_missing(&estimate.path));
            state.completed_paths += 1;
            state.completed_bytes += estimate.estimated_bytes.unwrap_or_default();
            on_progress(CleanupProgress {
                current_label: item.label.clone(),
                completed_bytes: state.completed_bytes,
                total_bytes: state.total_bytes,
                completed_paths: state.completed_paths,
                total_paths: state.total_paths,
            });
            continue;
        }
        match compute_path_size(&estimate.path) {
            Ok(size) => reclaimed += size,
            Err(error) => {
                skipped.push(language.path_size_failed(&estimate.path, &error.to_string()))
            }
        }
        if let Err(error) = remove_path_contents(&estimate.path) {
            skipped.push(language.path_delete_failed(&estimate.path, &error.to_string()));
        }
        state.completed_paths += 1;
        state.completed_bytes += estimate.estimated_bytes.unwrap_or_default();
        on_progress(CleanupProgress {
            current_label: item.label.clone(),
            completed_bytes: state.completed_bytes,
            total_bytes: state.total_bytes,
            completed_paths: state.completed_paths,
            total_paths: state.total_paths,
        });
    }
    CleanupOutcome {
        label: item.label.clone(),
        bytes_reclaimed: reclaimed,
        skipped,
    }
}

fn command_path(runner: &impl CommandRunner, program: &str, args: &[&str]) -> Option<PathBuf> {
    runner.run(program, args).ok().and_then(|output| {
        let trimmed = output.stdout.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(PathBuf::from(trimmed))
        }
    })
}

fn parse_docker_reclaimable(stdout: &str) -> Option<u64> {
    for line in stdout.lines() {
        if !line.contains("Build Cache") {
            continue;
        }
        if let Some(reclaimable_idx) = line.find("\"Reclaimable\":\"") {
            let remainder = &line[reclaimable_idx + "\"Reclaimable\":\"".len()..];
            let value = remainder.split('"').next()?;
            let bytes_text = value.split_whitespace().next()?;
            return parse_size_to_bytes(bytes_text);
        }
    }
    None
}

pub fn parse_size_to_bytes(value: &str) -> Option<u64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let boundary = trimmed
        .find(|ch: char| !matches!(ch, '0'..='9' | '.'))
        .unwrap_or(trimmed.len());
    let number: f64 = trimmed[..boundary].parse().ok()?;
    let unit = trimmed[boundary..].trim().to_ascii_uppercase();
    let multiplier = match unit.as_str() {
        "" | "B" => 1_f64,
        "KB" | "KIB" => 1_000_f64,
        "MB" | "MIB" => 1_000_000_f64,
        "GB" | "GIB" => 1_000_000_000_f64,
        "TB" | "TIB" => 1_000_000_000_000_f64,
        _ => return None,
    };
    Some((number * multiplier).round() as u64)
}

pub fn compute_path_size(path: &Path) -> Result<u64> {
    if !path.exists() {
        return Ok(0);
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Ok(0);
    }
    if metadata.is_file() {
        return Ok(metadata.len());
    }

    let mut total = 0;
    for entry in WalkDir::new(path).follow_links(false) {
        let entry = entry?;
        let file_type = entry.file_type();
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_file() {
            total += entry.metadata()?.len();
        }
    }
    Ok(total)
}

fn remove_path_contents(path: &Path) -> Result<()> {
    if path.is_file() {
        fs::remove_file(path)?;
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let child_path = entry.path();
        let metadata = fs::symlink_metadata(&child_path)?;
        if metadata.is_dir() {
            fs::remove_dir_all(&child_path)?;
        } else {
            fs::remove_file(&child_path)?;
        }
    }
    Ok(())
}
