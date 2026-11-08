use std::fs;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const DEFAULT_MODEL: &str = "deepseek-chat";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Resolve `$XDG_CONFIG_HOME/askds/config.json`, falling back to
/// `~/.config/askds/config.json`.
pub fn config_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"));
    base.join("askds").join("config.json")
}

pub fn load() -> Result<Config> {
    let path = config_path();
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(err) => return Err(err).with_context(|| format!("could not read {}", path.display())),
    };

    serde_json::from_str(&raw).with_context(|| format!("invalid config file {}", path.display()))
}

/// Write the config file with owner-only permissions.
pub fn save(config: &Config) -> Result<PathBuf> {
    let path = config_path();
    let dir = path.parent().expect("config path always has a parent");
    fs::create_dir_all(dir).with_context(|| format!("could not create {}", dir.display()))?;

    let json = serde_json::to_string_pretty(config)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

        // The file contains an API key, so make sure it is never world readable,
        // even if it already existed with looser permissions.
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)
            .with_context(|| format!("could not open {}", path.display()))?;
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
        file.write_all(json.as_bytes())?;
        file.write_all(b"\n")?;
        file.flush()?;
    }

    #[cfg(not(unix))]
    {
        fs::write(&path, format!("{json}\n"))
            .with_context(|| format!("could not write {}", path.display()))?;
    }

    Ok(path)
}
