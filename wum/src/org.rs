use crate::types::TodoItem;
use anyhow::Result;
use regex::Regex;

pub async fn read_todos(org_file: &std::path::Path) -> Result<Vec<TodoItem>> {
    let content = tokio::fs::read_to_string(org_file).await?;
    let re_todo = Regex::new(r"^(\*+)\s+TODO\s+([^!]+)!(\d+)\s*$").unwrap();
    let mut todos = Vec::new();
    for line in content.lines() {
        if let Some(caps) = re_todo.captures(line) {
            todos.push(TodoItem {
                repo: caps[2].to_string(),
                iid: caps[3].to_string(),
            });
        }
    }
    Ok(todos)
}

pub async fn mark_done_in_file(org_file: &std::path::Path, repo: &str, iid: &str) {
    let re_todo = Regex::new(r"^(\*+)\s+TODO\s+([^!]+)!(\d+)\s*$").unwrap();
    let re_bot_msg = Regex::new(r"^\s*(❌|⚠)\s").unwrap();
    let re_heading = Regex::new(r"^\*+\s+").unwrap();

    if let Ok(content) = tokio::fs::read_to_string(org_file).await {
        let mut lines: Vec<String> = content.lines().map(String::from).collect();
        let mut target_idx = None;
        for (i, line) in lines.iter().enumerate() {
            if let Some(caps) = re_todo.captures(line)
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
                if re_heading.is_match(&lines[end_idx]) {
                    break;
                }
                end_idx += 1;
            }

            let mut i = start_idx + 1;
            while i < end_idx {
                if re_bot_msg.is_match(&lines[i]) {
                    lines.remove(i);
                    end_idx -= 1;
                } else {
                    i += 1;
                }
            }

            lines[start_idx] = lines[start_idx].replacen("TODO", "DONE", 1);
            let _ = tokio::fs::write(org_file, lines.join("\n") + "\n").await;
        }
    }
}

pub async fn log_issue_in_file(org_file: &std::path::Path, repo: &str, iid: &str, message: &str) {
    let re_todo = Regex::new(r"^(\*+)\s+TODO\s+([^!]+)!(\d+)\s*$").unwrap();

    if let Ok(content) = tokio::fs::read_to_string(org_file).await {
        let mut lines: Vec<String> = content.lines().map(String::from).collect();
        let mut target_idx = None;
        for (i, line) in lines.iter().enumerate() {
            if let Some(caps) = re_todo.captures(line)
                && caps[2] == *repo
                && caps[3] == *iid
            {
                target_idx = Some(i);
                break;
            }
        }

        if let Some(start_idx) = target_idx {
            if start_idx + 1 < lines.len() && lines[start_idx + 1].trim() == message.trim() {
                return;
            }
            lines.insert(start_idx + 1, message.to_string());
            let _ = tokio::fs::write(org_file, lines.join("\n") + "\n").await;
        }
    }
}
