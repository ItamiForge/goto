use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use shellexpand::full;
use std::{fs, path::PathBuf};

const DEFAULT_CONFIG: &str = include_str!("../default_config.toml");

pub(crate) fn load_config() -> Result<ConfigFile> {
    let raw = load_config_raw()?;
    toml::from_str(&raw).context("invalid config TOML")
}

pub(crate) fn load_config_for_update() -> Result<ConfigFile> {
    load_config()
}

pub(crate) fn save_config(config: &ConfigFile) -> Result<()> {
    let path = config_file();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory {}", parent.display()))?;
    }

    let rendered = render_config(config)?;
    fs::write(&path, rendered).with_context(|| format!("failed to write {}", path.display()))
}

pub(crate) fn render_config(config: &ConfigFile) -> Result<String> {
    toml::to_string_pretty(config).context("failed to serialize config TOML")
}

pub(crate) fn config_file() -> PathBuf {
    match current_project_dirs() {
        Some(dirs) => dirs.config_dir().join("config.toml"),
        None => PathBuf::from("goto-config.toml"),
    }
}

pub(crate) fn expand_path(value: &str) -> PathBuf {
    let expanded = full(value).unwrap_or_else(|_| value.into());
    PathBuf::from(expanded.as_ref())
}

fn load_config_raw() -> Result<String> {
    let config_path = config_file();

    if config_path.exists() {
        return fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read {}", config_path.display()));
    }

    Ok(DEFAULT_CONFIG.to_string())
}

fn current_project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("io.github", "itamiforge", "goto")
}

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct ConfigFile {
    #[serde(rename = "namespace")]
    pub(crate) namespace: Vec<NamespaceDef>,
}

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct NamespaceDef {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) aliases: Option<Vec<String>>,
}
