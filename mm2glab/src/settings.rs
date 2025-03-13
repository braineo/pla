use std::{env, path::PathBuf};

use config::{Config, File};
use log::debug;
use serde::Deserialize;

use crate::cli::Args;

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub mm_url: Option<String>,
    pub mm_token: Option<String>,
    pub gitlab_url: Option<String>,
    pub gitlab_token: Option<String>,
    pub project_id: Option<String>,
    pub ollama_model: Option<String>,
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

pub fn merge_settings_with_args(args: &Args) -> anyhow::Result<Args> {
    let config_builder = Config::builder();

    let mut new_args = args.clone();

    let mut settings = Settings {
        mm_url: None,
        mm_token: None,
        gitlab_url: None,
        gitlab_token: None,
        project_id: None,
        ollama_model: None,
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

    // Apply config values to args if they're not already set by CLI
    // Using a macro to reduce repetition for string fields
    macro_rules! apply_if_empty {
        ($args:expr, $field:ident, $config:expr) => {
            if let Some(value) = $config.$field {
                if $args.$field.is_empty() {
                    $args.$field = value.clone();
                }
            }
        };
    }

    apply_if_empty!(new_args, mm_url, settings);
    apply_if_empty!(new_args, mm_token, settings);
    apply_if_empty!(new_args, gitlab_url, settings);
    apply_if_empty!(new_args, gitlab_token, settings);
    apply_if_empty!(new_args, project_id, settings);

    if let Some(ollama_model) = settings.ollama_model {
        if !ollama_model.is_empty() && new_args.ollama_model == "deepseek-r1:latest" {
            new_args.ollama_model = ollama_model;
        }
    }

    debug!("merged config: {:?}", new_args);

    Ok(new_args)
}
