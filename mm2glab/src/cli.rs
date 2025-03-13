use std::fmt;

use clap::{Parser, ValueEnum};
use log::LevelFilter;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Mattermost permanent link to the thread
    pub permalink: String,

    /// Custom issue title (optional)
    #[arg(long)]
    pub title: Option<String>,

    /// Mattermost server URL
    #[arg(long, env = "MATTERMOST_URL")]
    pub mm_url: String,

    /// Mattermost access token
    #[arg(long, env = "MATTERMOST_TOKEN")]
    pub mm_token: String,

    /// GitLab server URL
    #[arg(long, env = "GITLAB_URL")]
    pub gitlab_url: String,

    /// GitLab access token
    #[arg(long, env = "GITLAB_TOKEN")]
    pub gitlab_token: String,

    /// GitLab project ID
    #[arg(long, env = "GITLAB_PROJECT_ID")]
    pub project_id: String,

    /// Disable reply in Mattermost thread
    #[arg(long)]
    pub no_reply: bool,

    /// Skip preview and editor
    #[arg(long)]
    pub no_preview: bool,

    /// Modal
    #[arg(long, default_value = "deepseek-r1:latest")]
    pub ollama_model: String,

    /// Log verbosity
    #[arg(short, long, value_name = "LEVEL", default_value_t = LogLevel::Info)]
    pub log_level: LogLevel,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Off,
}

impl From<LogLevel> for LevelFilter {
    fn from(log_level: LogLevel) -> Self {
        match log_level {
            LogLevel::Trace => LevelFilter::Trace,
            LogLevel::Debug => LevelFilter::Debug,
            LogLevel::Info => LevelFilter::Info,
            LogLevel::Warn => LevelFilter::Warn,
            LogLevel::Error => LevelFilter::Error,
            LogLevel::Off => LevelFilter::Off,
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "trace"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
            LogLevel::Off => write!(f, "off"),
        }
    }
}
