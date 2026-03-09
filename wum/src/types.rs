use clap::Parser;
use crossterm::event::Event;
use serde::Deserialize;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {
    /// Path to the org file to watch
    #[arg(default_value = "merge_todo.org")]
    pub org_file: PathBuf,

    /// Polling interval in seconds (used as a fallback if no file changes occur)
    #[arg(short, long, default_value_t = 20)]
    pub interval: u64,
}

#[derive(Debug, Clone)]
pub struct TodoItem {
    pub repo: String,
    pub iid: String,
}

#[derive(Deserialize, Debug)]
pub struct MrViewResponse {
    pub state: String,
    pub detailed_merge_status: Option<String>,
    pub web_url: Option<String>,
    pub source_branch: Option<String>,
    pub title: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct CiPipelineResponse {
    pub status: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct CiJobResponse {
    pub jobs: Option<Vec<CiJob>>,
}

#[derive(Deserialize, Debug)]
pub struct CiJob {
    pub id: Option<u64>,
    pub stage: Option<String>,
    pub status: Option<String>,
}

#[derive(Clone, Debug)]
pub struct MrState {
    pub repo: String,
    pub iid: String,
    pub title: String,
    pub url: String,
    pub status_text: String,
    pub started_at: Instant,
    pub done: bool,
    pub completed_in: Option<Duration>,
}

pub enum AppEvent {
    Tick,
    FileChanged,
    UiEvent(Event),
    MrStateUpdate {
        repo: String,
        iid: String,
        title: Option<String>,
        url: Option<String>,
        status_text: Option<String>,
        done: Option<bool>,
    },
    MarkDoneInFile {
        repo: String,
        iid: String,
    },
    LogIssueInFile {
        repo: String,
        iid: String,
        message: String,
    },
}
