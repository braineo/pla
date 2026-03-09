use crate::types::{AppEvent, CiJobResponse, CiPipelineResponse, MrViewResponse, TodoItem};
use anyhow::{Context, Result};
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::mpsc;

pub fn spawn_mr_processor(todo: TodoItem, tx: mpsc::UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        loop {
            match process_mr_once(&todo, &tx).await {
                Ok(true) => {
                    let _ = tx.send(AppEvent::MrStateUpdate {
                        repo: todo.repo.clone(),
                        iid: todo.iid.clone(),
                        title: None,
                        url: None,
                        status_text: Some("MERGED!".to_string()),
                        done: Some(true),
                    });
                    let _ = tx.send(AppEvent::MarkDoneInFile {
                        repo: todo.repo.clone(),
                        iid: todo.iid.clone(),
                    });
                    break;
                }
                Ok(false) => {
                    tokio::time::sleep(Duration::from_secs(10)).await;
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::MrStateUpdate {
                        repo: todo.repo.clone(),
                        iid: todo.iid.clone(),
                        title: None,
                        url: None,
                        status_text: Some(format!("Error: {e}")),
                        done: None,
                    });
                    tokio::time::sleep(Duration::from_secs(15)).await;
                }
            }
        }
    });
}

pub async fn process_mr_once(
    todo: &TodoItem,
    tx: &mpsc::UnboundedSender<AppEvent>,
) -> Result<bool> {
    let output = Command::new("glab")
        .args(["mr", "view", &todo.iid, "-F", "json", "-R", &todo.repo])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("glab error: {}", stderr.trim());
    }

    let mr_info: MrViewResponse =
        serde_json::from_slice(&output.stdout).context("Failed to parse glab mr view JSON")?;

    if mr_info.state == "merged" {
        return Ok(true);
    }

    let detailed_status = mr_info.detailed_merge_status.as_deref().unwrap_or("");
    let url = mr_info.web_url.clone().unwrap_or_default();
    let source_branch = mr_info.source_branch.clone().unwrap_or_default();
    let title = mr_info.title.clone().unwrap_or_default();

    let _ = tx.send(AppEvent::MrStateUpdate {
        repo: todo.repo.clone(),
        iid: todo.iid.clone(),
        title: Some(title),
        url: Some(url.clone()),
        status_text: Some(format!("Status: {detailed_status}")),
        done: None,
    });

    if detailed_status == "need_rebase" {
        return handle_need_rebase(todo, tx, &url).await;
    }

    if detailed_status == "mergeable" {
        return handle_mergeable(todo, tx, &url).await;
    }

    handle_ci_status(todo, tx, &source_branch, &url, detailed_status).await?;

    Ok(false)
}

async fn handle_need_rebase(
    todo: &TodoItem,
    tx: &mpsc::UnboundedSender<AppEvent>,
    url: &str,
) -> Result<bool> {
    let _ = tx.send(AppEvent::MrStateUpdate {
        repo: todo.repo.clone(),
        iid: todo.iid.clone(),
        title: None,
        url: None,
        status_text: Some("Rebasing...".to_string()),
        done: None,
    });

    let rebase_output = Command::new("glab")
        .args(["mr", "rebase", &todo.iid, "-R", &todo.repo])
        .output()
        .await?;

    if !rebase_output.status.success() {
        let _ = tx.send(AppEvent::LogIssueInFile {
            repo: todo.repo.clone(),
            iid: todo.iid.clone(),
            message: format!("❌ Rebase failed for MR {url} - needs manual fix"),
        });
        anyhow::bail!("Rebase failed");
    }
    Ok(false)
}

async fn handle_mergeable(
    todo: &TodoItem,
    tx: &mpsc::UnboundedSender<AppEvent>,
    url: &str,
) -> Result<bool> {
    let _ = tx.send(AppEvent::MrStateUpdate {
        repo: todo.repo.clone(),
        iid: todo.iid.clone(),
        title: None,
        url: None,
        status_text: Some("Merging...".to_string()),
        done: None,
    });

    let merge_output = Command::new("glab")
        .args([
            "mr",
            "merge",
            &todo.iid,
            "-R",
            &todo.repo,
            "--remove-source-branch",
            "--squash",
            "--auto-merge",
            "--yes",
        ])
        .output()
        .await?;

    if merge_output.status.success() {
        Ok(true)
    } else {
        let _ = tx.send(AppEvent::LogIssueInFile {
            repo: todo.repo.clone(),
            iid: todo.iid.clone(),
            message: format!("❌ Merge failed for MR {url}"),
        });
        anyhow::bail!("Merge failed");
    }
}

async fn handle_ci_status(
    todo: &TodoItem,
    tx: &mpsc::UnboundedSender<AppEvent>,
    source_branch: &str,
    url: &str,
    detailed_status: &str,
) -> Result<()> {
    let ci_get_output = Command::new("glab")
        .args([
            "ci",
            "get",
            "--branch",
            source_branch,
            "-F",
            "json",
            "-R",
            &todo.repo,
        ])
        .output()
        .await?;

    if let Ok(ci_info) = serde_json::from_slice::<CiPipelineResponse>(&ci_get_output.stdout) {
        let pipeline_status = ci_info.status.as_deref().unwrap_or("");

        if pipeline_status == "manual" {
            let _ = tx.send(AppEvent::MrStateUpdate {
                repo: todo.repo.clone(),
                iid: todo.iid.clone(),
                title: None,
                url: None,
                status_text: Some("Triggering manual pipeline...".to_string()),
                done: None,
            });

            let ci_jobs = Command::new("glab")
                .args([
                    "ci",
                    "get",
                    "--branch",
                    source_branch,
                    "-F",
                    "json",
                    "-R",
                    &todo.repo,
                ])
                .output()
                .await?;

            let mut triggered = false;
            if let Ok(jobs_info) = serde_json::from_slice::<CiJobResponse>(&ci_jobs.stdout)
                && let Some(jobs) = jobs_info.jobs
            {
                for job in jobs {
                    if job.stage.as_deref() == Some("build")
                        && job.status.as_deref() == Some("manual")
                        && let Some(jid) = job.id
                    {
                        let trigger = Command::new("glab")
                            .args(["ci", "trigger", &jid.to_string(), "-R", &todo.repo])
                            .output()
                            .await?;
                        if trigger.status.success() {
                            triggered = true;
                            break;
                        }
                    }
                }
            }

            if !triggered {
                let _ = tx.send(AppEvent::LogIssueInFile {
                    repo: todo.repo.clone(),
                    iid: todo.iid.clone(),
                    message: format!("❌ Check what pipeline needs manual trigger for MR {url}"),
                });
                anyhow::bail!("Manual action required");
            }
        } else if pipeline_status == "running" {
            let _ = tx.send(AppEvent::MrStateUpdate {
                repo: todo.repo.clone(),
                iid: todo.iid.clone(),
                title: None,
                url: None,
                status_text: Some("Pipeline running...".to_string()),
                done: None,
            });
        } else if pipeline_status == "failed" {
            let _ = tx.send(AppEvent::LogIssueInFile {
                repo: todo.repo.clone(),
                iid: todo.iid.clone(),
                message: format!("❌ Pipeline failed for MR {url} - needs manual fix"),
            });
            anyhow::bail!("Pipeline failed");
        } else {
            let msg = match detailed_status {
                "not_approved" => "Waiting for approval",
                "ci_must_pass" | "ci_still_running" => "Waiting for CI",
                other => other,
            };
            let _ = tx.send(AppEvent::MrStateUpdate {
                repo: todo.repo.clone(),
                iid: todo.iid.clone(),
                title: None,
                url: None,
                status_text: Some(msg.to_string()),
                done: None,
            });
        }
    }
    Ok(())
}
