use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Conversation {
    pub username: String,
    pub timestamp: DateTime<Local>,
    pub message: String,
    pub file_ids: Option<Vec<String>>,
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

#[derive(Debug, Deserialize, Clone)]
pub struct GitLabIssue {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub iid: u64,
    pub title: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub web_url: String,
    // Add other fields from GitLab API as needed
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assignee_ids: Vec<u64>,
}

// Create/Update issue request that allows optional fields
#[derive(Debug, Serialize, Default)]
pub struct GitLabIssueChangeset {
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

impl GitLabIssueChangeset {
    // Create an empty changeset for updates
    pub fn new() -> Self {
        Self::default()
    }

    // Create a changeset with required fields for new issues
    pub fn new_issue(title: String, description: String) -> Self {
        Self {
            title: Some(title),
            description: Some(description),
            ..Self::default()
        }
    }

    // Convenience methods to add fields
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

    // Add arbitrary field
    pub fn with_field<T: Into<serde_json::Value>>(mut self, key: &str, value: T) -> Self {
        self.extra_fields.insert(key.to_string(), value.into());
        self
    }
}

// Convert from GitLabIssue to GitLabIssueChangeset (for updates)
impl From<&GitLabIssue> for GitLabIssueChangeset {
    fn from(issue: &GitLabIssue) -> Self {
        Self {
            title: Some(issue.title.clone()),
            description: Some(issue.description.clone()),
            labels: if issue.labels.is_empty() {
                None
            } else {
                Some(issue.labels.clone())
            },
            assignee_ids: if issue.assignee_ids.is_empty() {
                None
            } else {
                Some(issue.assignee_ids.clone())
            },
            extra_fields: HashMap::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GitLabUploadResponse {
    pub url: String,
    pub markdown: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GitLabUser {
    pub id: u64,
    pub username: String,
    pub name: String,
    pub locked: bool,
    pub state: String,
    pub avatar_url: String,
    pub web_url: String,
}
