pub mod gitlab;
pub mod mattermost;

use chrono::{DateTime, Local};
use mattermost::PostMetaFile;

#[derive(Debug, Clone)]
pub struct Conversation {
    pub username: String,
    pub timestamp: DateTime<Local>,
    pub message: String,
    pub file_ids: Vec<String>,
    pub file_meta: Vec<PostMetaFile>,
}
