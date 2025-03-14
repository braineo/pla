pub mod gitlab;
pub mod mattermost;

use chrono::{DateTime, Local};

#[derive(Debug, Clone)]
pub struct Conversation {
    pub username: String,
    pub timestamp: DateTime<Local>,
    pub message: String,
    pub file_ids: Option<Vec<String>>,
}
