use crate::models::*;
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::{header, Client};
use std::time::Duration;

#[async_trait]
pub trait MattermostApi {
    async fn get_thread(&self, post_id: &str) -> Result<MattermostThread>;
    async fn get_user(&self, user_id: &str) -> Result<MattermostUser>;
    async fn create_post(
        &self,
        channel_id: &str,
        message: &str,
        root_id: Option<&str>,
    ) -> Result<()>;
    async fn download_file(&self, file_id: &str) -> Result<(String, Vec<u8>, String)>;
    async fn get_post(&self, post_id: &str) -> Result<MattermostPost>;
}

pub struct MattermostClient {
    client: Client,
    base_url: String,
    _token: String,
}

impl MattermostClient {
    pub fn new(base_url: String, token: String) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
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
        }
    }

    pub fn parse_permalink(permalink: &str) -> Result<(String, String)> {
        let re = regex::Regex::new(r"/([^/]+)/pl/([a-zA-Z0-9]+)")?;
        let caps = re
            .captures(permalink)
            .context("Invalid Mattermost permalink format")?;
        Ok((caps[1].to_string(), caps[2].to_string()))
    }
}

#[async_trait]
impl MattermostApi for MattermostClient {
    async fn get_thread(&self, post_id: &str) -> Result<MattermostThread> {
        let url = format!("{}/api/v4/posts/{}/thread", self.base_url, post_id);
        let response = self.client.get(&url).send().await?.json().await?;
        Ok(response)
    }

    async fn get_user(&self, user_id: &str) -> Result<MattermostUser> {
        let url = format!("{}/api/v4/users/{}", self.base_url, user_id);
        let response = self.client.get(&url).send().await?.json().await?;
        Ok(response)
    }

    async fn create_post(
        &self,
        channel_id: &str,
        message: &str,
        root_id: Option<&str>,
    ) -> Result<()> {
        let url = format!("{}/api/v4/posts", self.base_url);
        let mut body = serde_json::json!({
            "channel_id": channel_id,
            "message": message,
        });

        if let Some(root_id) = root_id {
            body["root_id"] = serde_json::Value::String(root_id.to_string());
        }

        self.client.post(&url).json(&body).send().await?;
        Ok(())
    }

    async fn download_file(&self, file_id: &str) -> Result<(String, Vec<u8>, String)> {
        let url = format!("{}/api/v4/files/{}", self.base_url, file_id);
        let response = self.client.get(&url).send().await?;

        let content_disposition = response
            .headers()
            .get(header::CONTENT_DISPOSITION)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("")
            .to_string();

        let filename = content_disposition
            .split("filename=")
            .nth(1)
            .unwrap_or("unknown")
            .trim_matches('"')
            .to_string();

        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        let bytes = response.bytes().await?.to_vec();
        Ok((filename, bytes, content_type))
    }

    async fn get_post(&self, post_id: &str) -> Result<MattermostPost> {
        let url = format!("{}/api/v4/posts/{}", self.base_url, post_id);
        let response = self.client.get(&url).send().await?.json().await?;
        Ok(response)
    }
}
