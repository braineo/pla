use crate::models::*;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::{header, multipart, Client};
use std::{path::Path, time::Duration};

#[async_trait]
pub trait GitLabApi {
    async fn create_issue(&self, issue: &GitLabIssue) -> Result<String>;
    async fn upload_file(&self, path: &Path) -> Result<GitLabUploadResponse>;
}

pub struct GitLabClient {
    client: Client,
    base_url: String,
    token: String,
    project_id: String,
}

impl GitLabClient {
    pub fn new(base_url: String, token: String, project_id: String) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "PRIVATE-TOKEN",
            header::HeaderValue::from_str(&token).unwrap(),
        );

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .default_headers(headers)
            .build()
            .unwrap();

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
            project_id,
        }
    }
}

#[async_trait]
impl GitLabApi for GitLabClient {
    async fn create_issue(&self, issue: &GitLabIssue) -> Result<String> {
        let url = format!(
            "{}/api/v4/projects/{}/issues",
            self.base_url, self.project_id
        );

        let response: serde_json::Value = self
            .client
            .post(&url)
            .json(&issue)
            .send()
            .await?
            .json()
            .await?;

        Ok(response["web_url"].as_str().unwrap_or_default().to_string())
    }

    async fn upload_file(&self, path: &Path) -> Result<GitLabUploadResponse> {
        let url = format!(
            "{}/api/v4/projects/{}/uploads",
            self.base_url, self.project_id
        );

        let file_name = path.file_name().unwrap().to_string_lossy();
        let file_part = multipart::Part::file(path).await?.file_name(file_name.to_string());

        let form = multipart::Form::new().part("file", file_part);

        let response: GitLabUploadResponse = self
            .client
            .post(&url)
            .multipart(form)
            .send()
            .await?
            .json()
            .await?;

        Ok(response)
    }
}
