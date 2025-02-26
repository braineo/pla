use crate::api::gitlab::{GitLabApi, GitLabClient};
use crate::api::mattermost::{MattermostApi, MattermostClient};
use crate::{cli::Args, models::*};

use anyhow::Result;
use chrono::{Local, TimeZone};
use dialoguer::Editor;
use indicatif::{ProgressBar, ProgressStyle};
use log::debug;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::Ollama;
use regex::Regex;
use std::collections::HashMap;
use std::time::Duration;
use tempfile::TempDir;
use termimad::{self, MadSkin};

const ISSUE_TEMPLATE: &str = r#"
**Source**: {source_link}

## Description
{description}

{reason}
"#;

pub async fn run(args: Args) -> Result<()> {
    let mm_client = MattermostClient::new(args.mm_url, args.mm_token);
    let gitlab_client = GitLabClient::new(args.gitlab_url, args.gitlab_token, args.project_id);

    let (_team_name, post_id) = MattermostClient::parse_permalink(&args.permalink)?;
    let thread = mm_client.get_thread(&post_id).await?;

    let conversation = get_conversation_from_thread(&thread, &post_id, &mm_client).await?;

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner} {msg}")
            .unwrap(),
    );
    spinner.set_message("Generating title and description from LLM...");
    spinner.enable_steady_tick(Duration::from_millis(100));

    let (ai_title, ai_description, ai_reason) =
        analyze_conversation(&conversation, args.ollama_model).await?;

    spinner.finish_and_clear();

    let title = args.title.unwrap_or(ai_title);

    let description = format_issue_description(&args.permalink, &ai_description, &ai_reason);

    let (final_title, final_description) = if !args.no_preview {
        preview_and_confirm(&title, &description)?
    } else {
        (title, description)
    };

    let conversation_markdown =
        format_conversation_and_attachments(&conversation, &mm_client, &gitlab_client).await?;

    let issue = GitLabIssueChangeset::new_issue(
        final_title.clone(),
        format!("{final_description}\n\n{conversation_markdown}"),
    );

    let issue = gitlab_client.create_issue(&issue).await?;
    println!("Successfully created issue: {}", issue.web_url);

    if !args.no_reply {
        let post = mm_client.get_post(&post_id).await?;
        let reply = format!(
            ":gitlab: This conversation is now tracked in GitLab Issue: [{}]({})",
            final_title, issue.web_url
        );
        mm_client
            .create_post(&post.channel_id, &reply, Some(&post_id))
            .await?;
        println!("Successfully posted reply in Mattermost thread");
    }

    Ok(())
}

async fn get_conversation_from_thread(
    thread: &MattermostThread,
    target_post_id: &str,
    mm_client: &impl MattermostApi,
) -> Result<Vec<Conversation>> {
    let target_post = thread
        .posts
        .get(target_post_id)
        .ok_or_else(|| anyhow::anyhow!("Target post not found"))?;
    let target_timestamp = target_post.create_at;

    let mut user_cache: HashMap<String, String> = HashMap::new();
    let mut conversations = Vec::new();

    // Iterate through posts in order using the thread's order field
    for post_id in &thread.order {
        if let Some(post) = thread.posts.get(post_id) {
            // Only include posts at or after the target timestamp
            if post.create_at >= target_timestamp {
                let user_id = &post.user_id;
                let username = if let Some(username) = user_cache.get(user_id) {
                    username.clone()
                } else {
                    // Fetch and cache user details
                    let user = mm_client.get_user(user_id).await?;
                    let username = match (user.first_name, user.last_name) {
                        (Some(first), Some(last)) => {
                            if (!first.is_empty()) && (!last.is_empty()) {
                                format!("{} {}", first, last)
                            } else {
                                user.username
                            }
                        }
                        (Some(first), None) => first,
                        (None, Some(last)) => last,
                        (None, None) => user.username,
                    };
                    user_cache.insert(user_id.clone(), username.clone());
                    username
                };

                conversations.push(Conversation {
                    username,
                    timestamp: Local
                        .timestamp_millis_opt(post.create_at)
                        .single()
                        .ok_or_else(|| anyhow::anyhow!("Invalid timestamp"))?,
                    message: post.message.clone(),
                    file_ids: post.file_ids.clone(),
                });
            }
        }
    }

    Ok(conversations)
}

async fn analyze_conversation(
    conversation: &[Conversation],
    ollama_model: String,
) -> Result<(String, String, String)> {
    let formatted_conv: String = conversation
        .iter()
        .map(|c| format!("{}: {}", c.username, c.message))
        .collect::<Vec<_>>()
        .join("\n");

    let ollama = Ollama::default();
    let prompt = format!(
        "Given this conversation, create a concise issue title and description for a developer issue.\n\n\
Conversation:\n\
{}\n\n\
Respond in this exact format with nothing else.\n\
title: <Issue Title in exactly one line>\n\
description: <Issue Description that can take multiple lines>",
        formatted_conv
    );
    debug!("feeding prompt to LLM:\n{prompt}");

    let req = GenerationRequest::new(ollama_model, prompt);
    let response = ollama.generate(req).await?;

    let content = response.response;
    debug!("received response:\n{content}");

    let think_regex = Regex::new(r"(?ms)<think>(.*?)</think>\n?")?;

    let reason = think_regex
        .captures(&content)
        .and_then(|cap| cap.get(1))
        .map_or_else(String::new, |m| m.as_str().trim().to_string());

    let content = think_regex.replace_all(&content, "").trim().to_string();

    let mut lines = content.lines();

    let title = lines
        .next()
        .map(|line| line.trim_start_matches("title:").trim())
        .unwrap_or("Untitled Issue")
        .to_string();

    let description = lines
        .collect::<Vec<_>>()
        .join("\n")
        .trim_start_matches("description:")
        .trim()
        .to_string();

    let description = if description.is_empty() {
        "No description provided.".to_string()
    } else {
        description
    };

    Ok((title, description, reason))
}

async fn format_conversation_and_attachments(
    conversations: &[Conversation],
    mm_client: &impl MattermostApi,
    gitlab_client: &impl GitLabApi,
) -> Result<String> {
    let temp_dir = TempDir::new()?;
    let mut markdown_lines = Vec::new();

    let progress = ProgressBar::new(
        conversations
            .iter()
            .filter(|c| c.file_ids.is_some())
            .map(|p| p.file_ids.as_ref().unwrap().len())
            .sum::<usize>() as u64,
    );

    for post in conversations.iter() {
        markdown_lines.push(format_conversation(post));

        if let Some(file_ids) = &post.file_ids {
            for file_id in file_ids {
                match mm_client.download_file(file_id).await {
                    Ok((filename, content, content_type)) => {
                        let file_path = temp_dir.path().join(&filename);
                        tokio::fs::write(&file_path, &content).await?;

                        match gitlab_client.upload_file(&file_path).await {
                            Ok(upload) => {
                                if content_type.starts_with("image/")
                                    || content_type.starts_with("video/")
                                {
                                    markdown_lines
                                        .push(format!("{}{{width=60%}}\n", upload.markdown));
                                } else {
                                    markdown_lines
                                        .push(format!("- [{}]({})\n", filename, upload.url));
                                }
                            }
                            Err(e) => {
                                eprintln!(
                                    "Failed to upload file {}: {}, use mattermost link instead",
                                    file_id, e
                                );
                                markdown_lines.push(format!(
                                    "- [{}]({})\n",
                                    filename,
                                    mm_client.get_file_url(file_id)
                                ));
                            }
                        }

                        progress.inc(1);
                    }
                    Err(e) => eprintln!("Failed to download file {}: {}", file_id, e),
                }
            }
        }
    }

    progress.finish_and_clear();

    Ok(format!(
        "<details>\n\
<summary>Conversation Thread</summary>\n\n\
{}\n\
</details>",
        markdown_lines.join("\n\n")
    ))
}

fn format_conversation(conversation: &Conversation) -> String {
    format!(
        "**{}** ({}): {}",
        conversation.username,
        conversation.timestamp.format("%Y-%m-%d %H:%M:%S"),
        conversation.message
    )
}

fn format_issue_description(source_link: &str, ai_description: &str, ai_reason: &str) -> String {
    ISSUE_TEMPLATE
        .replace("{source_link}", source_link)
        .replace(
            "{description}",
            &format!(">description generated by LLM based on the conversation\n\n{ai_description}"),
        )
        .replace(
            "{reason}",
            &format!(
                "<details>\n\
<summary>Think</summary>\n\n\
{ai_reason}\n\
</details>",
            ),
        )
}

fn preview_and_confirm(title: &str, description: &str) -> Result<(String, String)> {
    let skin = MadSkin::default();

    println!(
        "{}",
        skin.term_text(&format!(
            "\n---\nIssue Preview\n---\n# {title}\n\n{description}"
        ))
    );

    loop {
        let choice = dialoguer::Select::new()
            .with_prompt("What would you like to do?")
            .items(&["Proceed", "Edit", "Cancel"])
            .default(0)
            .interact()?;

        match choice {
            0 => return Ok((title.to_string(), description.to_string())),
            1 => {
                if let Ok(Some(edited_content)) = Editor::new().extension(".md").edit(&format!(
                    "Title: {}\n{}\n\n{}",
                    title,
                    "=".repeat(80),
                    description
                )) {
                    let lines: Vec<&str> = edited_content.lines().collect();
                    let new_title = lines[0].replace("Title: ", "").trim().to_string();
                    let new_description = lines[2..].join("\n");
                    return Ok((new_title, new_description));
                }
            }
            2 => return Err(anyhow::anyhow!("Operation cancelled by user")),
            _ => unreachable!(),
        }
    }
}
