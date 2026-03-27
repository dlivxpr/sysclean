use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow};

use crate::i18n::Language;

pub fn home_dir(language: Language) -> Result<PathBuf> {
    dirs::home_dir().ok_or_else(|| anyhow!(language.home_dir_not_found()))
}

pub fn local_app_data_dir(language: Language) -> Result<PathBuf> {
    dirs::data_local_dir().ok_or_else(|| anyhow!(language.local_app_data_not_found()))
}

pub fn app_cache_file(language: Language) -> Result<PathBuf> {
    let base = dirs::cache_dir()
        .or_else(dirs::data_local_dir)
        .ok_or_else(|| anyhow!(language.app_cache_dir_not_found()))?;
    Ok(base.join("sysclean").join("scan-cache.json"))
}

pub fn open_in_explorer(path: &Path, language: Language) -> Result<()> {
    Command::new("explorer")
        .arg(path)
        .status()
        .with_context(|| language.open_in_explorer_failed(path))?;
    Ok(())
}
