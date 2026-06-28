use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

/// CLI configuration loaded from TOML and environment.
#[allow(dead_code)]
#[derive(Debug, Default, Clone, Deserialize)]
pub struct Config {
    pub ollama_url: Option<String>,
    pub default_model: Option<String>,
    pub agents_dir: Option<PathBuf>,
    pub prompts_dir: Option<PathBuf>,
}

impl Config {
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let base = path
            .map(PathBuf::from)
            .or_else(default_path);

        if let Some(path) = base {
            if path.exists() {
                let raw = fs::read_to_string(&path)
                    .with_context(|| format!("reading config from {}", path.display()))?;
                let cfg: Config = toml::from_str(&raw)
                    .with_context(|| format!("parsing config at {}", path.display()))?;
                return Ok(cfg);
            }
        }

        Ok(Config::default())
    }
}

fn default_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok().map(PathBuf::from)?;
    Some(home.join(".airllm").join("config.toml"))
}
