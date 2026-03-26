use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow};

pub fn home_dir() -> Result<PathBuf> {
    dirs::home_dir().ok_or_else(|| anyhow!("无法定位当前用户目录"))
}

pub fn local_app_data_dir() -> Result<PathBuf> {
    dirs::data_local_dir().ok_or_else(|| anyhow!("无法定位 LocalAppData 目录"))
}

pub fn app_cache_file() -> Result<PathBuf> {
    let base = dirs::cache_dir()
        .or_else(dirs::data_local_dir)
        .ok_or_else(|| anyhow!("无法定位应用缓存目录"))?;
    Ok(base.join("sysclean").join("scan-cache.json"))
}

pub fn open_in_explorer(path: &Path) -> Result<()> {
    Command::new("explorer")
        .arg(path)
        .status()
        .with_context(|| format!("无法打开资源管理器: {}", path.display()))?;
    Ok(())
}
