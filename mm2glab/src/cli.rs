use std::fmt;

use clap::{Args, Parser, Subcommand, ValueEnum};
use log::LevelFilter;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// Generate a GitLab issue from a Mattermost thread (full pipeline)
    Issue(IssueArgs),
    /// Fetch a Mattermost thread and its media as local markdown for external agents
    Fetch(FetchArgs),
}

/// Mattermost connection details and logging, shared by all subcommands
#[derive(Args, Debug, Clone)]
pub struct CommonArgs {
    /// Mattermost permanent link to the thread
    pub permalink: String,

    /// Mattermost server URL
    #[arg(
        long,
        env = "MATTERMOST_URL",
        default_value = "",
        hide_default_value = true
    )]
    pub mm_url: String,

    /// Mattermost access token
    #[arg(
        long,
        env = "MATTERMOST_TOKEN",
        default_value = "",
        hide_default_value = true
    )]
    pub mm_token: String,

    /// Log verbosity
    #[arg(short, long, value_name = "LEVEL", default_value_t = LogLevel::Info)]
    pub log_level: LogLevel,
}

#[derive(Args, Debug, Clone)]
pub struct IssueArgs {
    #[command(flatten)]
    pub common: CommonArgs,

    /// Custom issue title (optional)
    #[arg(long)]
    pub title: Option<String>,

    /// GitLab server URL
    #[arg(
        long,
        env = "GITLAB_URL",
        default_value = "",
        hide_default_value = true
    )]
    pub gitlab_url: String,

    /// GitLab access token
    #[arg(
        long,
        env = "GITLAB_TOKEN",
        default_value = "",
        hide_default_value = true
    )]
    pub gitlab_token: String,

    /// GitLab project ID
    #[arg(
        long,
        env = "GITLAB_PROJECT_ID",
        default_value = "",
        hide_default_value = true
    )]
    pub project_id: String,

    /// Disable reply in Mattermost thread
    #[arg(long)]
    pub no_reply: bool,

    /// Skip preview and editor
    #[arg(long)]
    pub no_preview: bool,

    /// Ollama model to use for AI-powered issue generation
    #[arg(long, default_value = "deepseek-r1:latest")]
    pub ollama_model: String,

    /// External command to generate title/description instead of Ollama.
    /// The prompt is piped to the command's stdin and its stdout is parsed.
    /// Example: --agent-command "claude -p"
    #[arg(long, default_value = "", hide_default_value = true)]
    pub agent_command: String,

    /// Custom prompt template for LLM
    #[arg(long, default_value = "", hide_default_value = true)]
    pub prompt: String,
}

#[derive(Args, Debug, Clone)]
pub struct FetchArgs {
    #[command(flatten)]
    pub common: CommonArgs,

    /// Directory to write downloaded media into. Defaults to a timestamped
    /// directory under the system temp dir. This directory persists.
    #[arg(long)]
    pub output_dir: Option<String>,
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
