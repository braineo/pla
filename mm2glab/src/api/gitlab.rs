use anyhow::Result;
use async_trait::async_trait;
use reqwest::{header, multipart, Client};
use std::{path::Path, time::Duration};

use crate::models::{GitLabIssue, GitLabIssueChangeset, GitLabUploadResponse, GitLabUser};

#[async_trait]
pub trait GitLabApi {
    async fn create_issue(&self, issue: &GitLabIssueChangeset) -> Result<GitLabIssue>;
    async fn update_issue(
        &self,
        issue_id: u64,
        changeset: &GitLabIssueChangeset,
    ) -> Result<GitLabIssue>;
    async fn upload_file(&self, path: &Path) -> Result<GitLabUploadResponse>;
    async fn search_project_members(&self, search_term: &str) -> Result<Vec<GitLabUser>>;
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
            _token: token,
            project_id,
        }
    }
}

#[async_trait]
impl GitLabApi for GitLabClient {
    async fn create_issue(&self, issue: &GitLabIssueChangeset) -> Result<GitLabIssue> {
        if issue.title.is_none() || issue.description.is_none() {
            return Err(anyhow::anyhow!(
                "Title and description are required for new issues"
            ));
        }

        let url = format!(
            "{}/api/v4/projects/{}/issues",
            self.base_url, self.project_id
        );

        let response = self.client.post(&url).json(&issue).send().await?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!(
                "cannot create issue with status {}: {}",
                status,
                error_text
            ));
        };

        let issue: GitLabIssue = response.json().await?;

        Ok(issue)
    }

    async fn update_issue(
        &self,
        issue_id: u64,
        changeset: &GitLabIssueChangeset,
    ) -> Result<GitLabIssue> {
        let url = format!(
            "{}/api/v4/projects/{}/issues/{}",
            self.base_url, self.project_id, issue_id
        );

        let response = self.client.put(&url).json(&changeset).send().await?;
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!(
                "Failed to update issue: {} - {}",
                status,
                error_text
            ));
        }

        let issue: GitLabIssue = response.json().await?;

        Ok(issue)
    }

    async fn search_project_members(&self, search_term: &str) -> Result<Vec<GitLabUser>> {
        let url = format!(
            "{}/api/v4/projects/{}/members",
            self.base_url, self.project_id
        );

        let response = self
            .client
            .get(&url)
            .query(&[("search", search_term), ("active", "true")])
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!(
                "cannot search user with status {}: {}",
                status,
                error_text
            ));
        };

        let members: Vec<GitLabUser> = response.json().await?;

        Ok(members)
    }

    async fn upload_file(&self, path: &Path) -> Result<GitLabUploadResponse> {
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
        let gitlab_response: GitLabUploadResponse = response.json().await?;

        Ok(gitlab_response)
    }
}
