pub mod gitlab;
pub mod mattermost;

use chrono::{DateTime, Local};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ImageFileInfo {
    pub file_id: String,
    pub filename: String,
    pub mime_type: String,
    pub analysis: Option<String>,
    pub is_key_media: bool,
}

#[derive(Debug, Clone)]
pub struct Conversation {
    pub username: String,
    pub timestamp: DateTime<Local>,
    pub message: String,
    pub file_ids: Option<Vec<String>>,
    pub image_files: Option<HashMap<String, ImageFileInfo>>,
}
