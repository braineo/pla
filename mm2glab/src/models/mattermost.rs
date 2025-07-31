use chrono::{DateTime, Local};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Conversation {
    pub username: String,
    pub timestamp: DateTime<Local>,
    pub message: String,
    pub file_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct Post {
    pub user_id: String,
    pub channel_id: String,
    pub message: String,
    pub create_at: i64,
    pub file_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct Thread {
    pub posts: HashMap<String, Post>,
    pub order: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub username: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}
