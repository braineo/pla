use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct Post {
    pub user_id: String,
    pub channel_id: String,
    pub message: String,
    pub create_at: i64,
    pub file_ids: Option<Vec<String>>,
    pub metadata: PostMetaData,
}

#[derive(Debug, Deserialize)]
pub struct PostMetaFile {
    pub id: String,
    pub mime_type: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct PostMetaData {
    pub files: Option<Vec<PostMetaFile>>,
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
