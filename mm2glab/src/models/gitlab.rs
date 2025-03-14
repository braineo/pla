use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct Issue {
    pub title: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct UploadResponse {
    pub url: String,
    pub markdown: String,
}
