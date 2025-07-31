use anyhow::Result;
use async_trait::async_trait;
use reqwest::{header, multipart, Client};
use std::{path::Path, time::Duration};

use crate::models::gitlab::{Issue, IssueChangeset, UploadResponse, User};

#[async_trait]
pub trait GitLabApi {
    async fn create_issue(&self, issue: &IssueChangeset) -> Result<Issue>;
    async fn update_issue(&self, issue_id: u64, changeset: &IssueChangeset) -> Result<Issue>;
    async fn upload_file(&self, path: &Path) -> Result<UploadResponse>;
    async fn search_project_members(&self, search_term: &str) -> Result<Vec<User>>;
}

#[derive(Clone)]
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

    async fn handle_response<T>(&self, response: reqwest::Response) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!(
                "GitLab API request failed with status {}: {}",
                status,
                error_text
            ));
        }
        response.json().await.map_err(Into::into)
    }
}

#[async_trait]
impl GitLabApi for GitLabClient {
    async fn create_issue(&self, issue: &IssueChangeset) -> Result<Issue> {
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
        let issue: Issue = self.handle_response(response).await?;

        Ok(issue)
    }

    async fn update_issue(&self, issue_id: u64, changeset: &IssueChangeset) -> Result<Issue> {
        let url = format!(
            "{}/api/v4/projects/{}/issues/{}",
            self.base_url, self.project_id, issue_id
        );

        let response = self.client.put(&url).json(&changeset).send().await?;
        let issue: Issue = self.handle_response(response).await?;

        Ok(issue)
    }

    async fn search_project_members(&self, search_term: &str) -> Result<Vec<User>> {
        let url = format!(
            "{}/api/v4/projects/{}/members/all",
            self.base_url, self.project_id
        );
        let mut all_members: Vec<User> = vec![];
        let mut page = 1;

        loop {
            let response = self
                .client
                .get(&url)
                .query(&[("query", search_term)])
                .query(&[("page", page)])
                .query(&[("per_page", 100)]) // Adjust per_page as needed
                .send()
                .await?;
            let members: Vec<User> = self.handle_response(response).await?;

            if members.is_empty() {
                break;
            } else {
                all_members.extend(members);
                page += 1;
            }
        }

        Ok(all_members
            .into_iter()
            // active and access level is developer or above
            .filter(|m| m.state == "active" && m.access_level >= 30)
            .collect())
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
        let gitlab_response: UploadResponse = self.handle_response(response).await?;

        Ok(gitlab_response)
    }
}
