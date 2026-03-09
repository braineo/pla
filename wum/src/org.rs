use crate::types::TodoItem;
use anyhow::Result;
use regex::Regex;
use std::sync::LazyLock;

static RE_TODO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\*+)\s+TODO\s+([^!]+)!(\d+)\s*$").unwrap());
static RE_BOT_MSG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*(❌|⚠)\s").unwrap());
static RE_HEADING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\*+\s+").unwrap());

pub async fn read_todos(org_file: &std::path::Path) -> Result<Vec<TodoItem>> {
    let content = tokio::fs::read_to_string(org_file).await?;
    let mut todos = Vec::new();
    for line in content.lines() {
        if let Some(caps) = RE_TODO.captures(line) {
            todos.push(TodoItem {
                repo: caps[2].to_string(),
                iid: caps[3].to_string(),
            });
        }
    }
    Ok(todos)
}

pub async fn mark_done_in_file(org_file: &std::path::Path, repo: &str, iid: &str) -> Result<()> {
    let content = tokio::fs::read_to_string(org_file).await?;
    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    let mut target_idx = None;
    for (i, line) in lines.iter().enumerate() {
        if let Some(caps) = RE_TODO.captures(line)
            && caps[2] == *repo
            && caps[3] == *iid
        {
            target_idx = Some(i);
            break;
        }
    }

    if let Some(start_idx) = target_idx {
        let mut end_idx = start_idx + 1;
        while end_idx < lines.len() {
            if RE_HEADING.is_match(&lines[end_idx]) {
                break;
            }
            end_idx += 1;
        }

        let mut i = start_idx + 1;
        while i < end_idx {
            if RE_BOT_MSG.is_match(&lines[i]) {
                lines.remove(i);
                end_idx -= 1;
            } else {
                i += 1;
            }
        }

        lines[start_idx] = lines[start_idx].replacen("TODO", "DONE", 1);
        tokio::fs::write(org_file, lines.join("\n") + "\n").await?;
    }
    Ok(())
}

pub async fn log_issue_in_file(
    org_file: &std::path::Path,
    repo: &str,
    iid: &str,
    message: &str,
) -> Result<()> {
    let content = tokio::fs::read_to_string(org_file).await?;
    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    let mut target_idx = None;
    for (i, line) in lines.iter().enumerate() {
        if let Some(caps) = RE_TODO.captures(line)
            && caps[2] == *repo
            && caps[3] == *iid
        {
            target_idx = Some(i);
            break;
        }
    }

    if let Some(start_idx) = target_idx {
        if start_idx + 1 < lines.len() && lines[start_idx + 1].trim() == message.trim() {
            return Ok(());
        }
        lines.insert(start_idx + 1, message.to_string());
        tokio::fs::write(org_file, lines.join("\n") + "\n").await?;
    }
    Ok(())
}
