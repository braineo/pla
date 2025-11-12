use std::{fmt, path::PathBuf};

use clap::{Parser, ValueEnum};
use log::LevelFilter;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Root directory to search for repositories
    #[arg(short, long, default_value = ".")]
    pub dir: PathBuf,

    /// Command to execute in each repository (e.g., "git pull && npm install")
    #[arg(short = 'C', long)]
    pub command: Option<String>,

    /// Command to execute in each repository
    #[arg(short, long)]
    pub command_file: Option<PathBuf>,

    /// Pattern to match repository names (e.g., "app.+")
    #[arg(short, long)]
    pub match_regexp: Option<String>,

    /// Command to list directories (e.g., "find . -type f  -maxdepth 2 -name "package.json" -printf '%P\n' | xargs -I {} dirname {}")
    /// If specified it will replace "ls"
    #[arg(short = 'L', long)]
    pub list_command: Option<String>,

    /// Select last failed repositories by default
    #[arg(short, long)]
    pub failed: bool,

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
