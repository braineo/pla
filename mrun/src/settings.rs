use anyhow::Result;
use config::{Config, File};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::{env, fs::OpenOptions, path::PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub last_selected_repos: Vec<String>,
    pub last_failed_repos: Vec<String>,
}

const CONFIG_FILE_NAME: &str = env!("CARGO_PKG_NAME");

// Function to get the XDG_CONFIG_HOME path
fn get_xdg_config_path() -> Option<PathBuf> {
    // First check XDG_CONFIG_HOME environment variable
    if let Ok(xdg_config) = env::var("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg_config));
    }

    // If XDG_CONFIG_HOME is not set, fall back to $HOME/.config
    if let Ok(home) = env::var("HOME") {
        return Some(PathBuf::from(home).join(".config"));
    }

    None
}

pub fn load_settings() -> anyhow::Result<Settings> {
    let config_builder = Config::builder();

    let mut settings = Settings {
        last_selected_repos: vec![],
        last_failed_repos: vec![],
    };

    if let Some(xdg_config) = get_xdg_config_path() {
        let config_path = xdg_config.join(CONFIG_FILE_NAME).join("config.toml");
        if config_path.exists() {
            settings = config_builder
                .add_source(File::from(config_path.clone()).required(false))
                .build()?
                .try_deserialize()
                .map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to deserialize config file {}: {}",
                        config_path.display(),
                        e
                    )
                })?
        }
    }

    Ok(settings)
}

pub fn write_settings(settings: &Settings) -> Result<()> {
    if let Some(xdg_config) = get_xdg_config_path() {
        let config_folder = xdg_config.join(CONFIG_FILE_NAME);
        fs::create_dir_all(&config_folder)?;

        let config_path = config_folder.join("config.toml");
        let toml_string = toml::to_string(settings)?;

        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(config_path)?;

        file.write_all(toml_string.as_bytes())?
    }

    Ok(())
}
