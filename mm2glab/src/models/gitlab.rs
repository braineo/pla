use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct Issue {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub iid: u64,
    pub title: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub web_url: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assignee_ids: Vec<u64>,
}

#[derive(Debug, Serialize, Default)]
pub struct IssueChangeset {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee_ids: Option<Vec<u64>>,

    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extra_fields: HashMap<String, serde_json::Value>,
}

impl IssueChangeset {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_issue(title: String, description: String) -> Self {
        Self {
            title: Some(title),
            description: Some(description),
            ..Self::default()
        }
    }

    pub fn with_title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn with_assignees(mut self, assignee_ids: Vec<u64>) -> Self {
        self.assignee_ids = Some(assignee_ids);
        self
    }

    pub fn with_field<T: Into<serde_json::Value>>(mut self, key: &str, value: T) -> Self {
        self.extra_fields.insert(key.to_string(), value.into());
        self
    }
}

impl From<&Issue> for IssueChangeset {
    fn from(issue: &Issue) -> Self {
        Self {
            title: Some(issue.title.clone()),
            description: Some(issue.description.clone()),
            labels: (!issue.labels.is_empty()).then_some(issue.labels.clone()),
            assignee_ids: (!issue.assignee_ids.is_empty()).then_some(issue.assignee_ids.clone()),
            extra_fields: HashMap::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UploadResponse {
    pub url: String,
    pub markdown: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub name: String,
    pub state: String,
    pub access_level: u16,
}

impl std::fmt::Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, ({})", self.name, self.username)
    }
}
