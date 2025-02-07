use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Conversation {
    pub username: String,
    pub timestamp: DateTime<Local>,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct MattermostPost {
    pub user_id: String,
    pub channel_id: String,
    pub message: String,
    pub create_at: i64,
    pub file_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct MattermostThread {
    pub posts: HashMap<String, MattermostPost>,
    pub order: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct MattermostUser {
    pub username: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GitLabIssue {
    pub title: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct GitLabUploadResponse {
    pub url: String,
    pub markdown: String,
}
