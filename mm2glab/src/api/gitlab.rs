use anyhow::Result;
use async_trait::async_trait;
use reqwest::{header, multipart, Client};
use std::{path::Path, time::Duration};

use crate::models::gitlab::{Issue, UploadResponse};

#[async_trait]
pub trait GitLabApi {
    async fn create_issue(&self, issue: &Issue) -> Result<String>;
    async fn upload_file(&self, path: &Path) -> Result<UploadResponse>;
}

pub struct GitLabClient {
    client: Client,
    base_url: String,
    _token: String,
    project_id: String,
}

impl GitLabClient {
    pub fn new(base_url: String, token: String, project_id: String) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "PRIVATE-TOKEN",
            header::HeaderValue::from_str(&token)
                .expect("Failed to create header value from token string"),
        );

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .default_headers(headers)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            _token: token,
            project_id,
        }
    }
}

#[async_trait]
impl GitLabApi for GitLabClient {
    async fn create_issue(&self, issue: &Issue) -> Result<String> {
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

    async fn upload_file(&self, path: &Path) -> Result<UploadResponse> {
        let url = format!(
            "{}/api/v4/projects/{}/uploads",
            self.base_url, self.project_id
        );

        let file_name = path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?
            .to_string_lossy();

        let file_part = multipart::Part::file(path)
            .await?
            .file_name(file_name.to_string());

        let form = multipart::Form::new().part("file", file_part);

        let response = self.client.post(&url).multipart(form).send().await?;

        // Check the status code before trying to parse JSON
        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!(
                "GitLab upload failed with status {}: {}",
                status,
                error_text
            ));
        }

        // Only try to parse as JSON if we got a success status
        let gitlab_response: UploadResponse = response.json().await?;

        Ok(gitlab_response)
    }
}
