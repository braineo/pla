use anyhow::Result;
use clap::Parser;
use console::Term;
use indicatif::{ProgressBar, ProgressStyle};
use log::warn;
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use owo_colors::OwoColorize;
use regex::Regex;
use serde_json::Value;
use std::{collections::HashSet, path::PathBuf, sync::mpsc, time::Duration};
use tokio::process::Command;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Path to the org file to watch
    #[arg(default_value = "merge_todo.org")]
    org_file: PathBuf,

    /// Polling interval in seconds (used as a fallback if no file changes occur)
    #[arg(short, long, default_value_t = 20)]
    interval: u64,

    /// Default branch (legacy from bash, but maybe useful if needed)
    #[arg(short, long, default_value = "update-dep")]
    default_branch: String,
}

#[derive(Debug, Clone)]
struct TodoItem {
    repo: String,
    iid: String,
}

fn spinner_style() -> ProgressStyle {
    ProgressStyle::default_spinner()
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
        .template("{spinner:.cyan} {msg}")
        .unwrap()
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    let args = Args::parse();
    let term = Term::stdout();

    let org_file = if args.org_file.is_absolute() {
        args.org_file.clone()
    } else {
        std::env::current_dir()?.join(&args.org_file)
    };

    let (tx, rx) = mpsc::channel();
    let mut debouncer_opt = match new_debouncer(Duration::from_secs(1), tx) {
        Ok(debouncer) => Some(debouncer),
        Err(e) => {
            warn!(
                "Failed to initialize file watcher: {}. Falling back to polling every {}s.",
                e, args.interval
            );
            None
        }
    };

    if org_file.exists()
        && let Some(parent) = org_file.parent()
            && let Some(ref mut debouncer) = debouncer_opt
                && let Err(e) = debouncer
                    .watcher()
                    .watch(parent, RecursiveMode::NonRecursive)
                {
                    warn!("Failed to watch directory {}: {}", parent.display(), e);
                }

    let interval = Duration::from_secs(args.interval);
    let mut iteration = 0;

    let re_todo = Regex::new(r"^(\*+)\s+TODO\s+([^!]+)!(\d+)\s*$").unwrap();
    let re_bot_msg = Regex::new(r"^\s*(❌|⚠)\s").unwrap();
    let re_heading = Regex::new(r"^\*+\s+").unwrap();

    loop {
        iteration += 1;
        term.clear_screen()?;
        println!(
            "{}",
            "╭────────────────────────────────────────────╮".cyan()
        );
        println!(
            "│  {} {}  │",
            "Iteration".bold(),
            format!(
                "{:02} - {}",
                iteration,
                chrono::Local::now().format("%H:%M:%S")
            )
            .bright_blue()
        );
        println!(
            "{}",
            "╰────────────────────────────────────────────╯\n".cyan()
        );

        if !org_file.exists() {
            println!(
                "{} {}",
                " ⚠ ".on_yellow().black().bold(),
                format!("{} not found.", org_file.display()).yellow()
            );
            println!("\n  You can create it by adding repos like this:");
            println!("    {}", "* TODO frontend/uicore!762".bright_black());
            println!("\n  {}...", "Waiting for file to be created".italic());

            wait_for_event(&rx, interval).await;

            if org_file.exists()
                && let Some(parent) = org_file.parent()
                    && let Some(ref mut debouncer) = debouncer_opt {
                        let _ = debouncer
                            .watcher()
                            .watch(parent, RecursiveMode::NonRecursive);
                    }
            continue;
        }

        let mut all_done = true;
        let mut processed_repos = HashSet::new();

        let file_content = match tokio::fs::read_to_string(&org_file).await {
            Ok(content) => content,
            Err(e) => {
                println!(
                    "{} Could not read file {}: {}",
                    " ❌ ".on_red().white().bold(),
                    org_file.display().red(),
                    e
                );
                wait_for_event(&rx, interval).await;
                continue;
            }
        };

        let mut lines: Vec<String> = file_content.lines().map(String::from).collect();
        let mut todos = Vec::new();

        for line in &lines {
            if let Some(caps) = re_todo.captures(line) {
                todos.push(TodoItem {
                    repo: caps[2].to_string(),
                    iid: caps[3].to_string(),
                });
            }
        }

        if todos.is_empty() {
            println!(
                "  {} No pending TODOs in {}!",
                "✓".green().bold(),
                org_file.display()
            );
            println!(
                "    {}...",
                "Waiting for new TODOs to be added".italic().bright_black()
            );
            println!(
                "\n  {} Next check in {} seconds (or when file changes)...",
                "⏱".cyan(),
                interval.as_secs()
            );
            wait_for_event(&rx, interval).await;
            continue;
        }

        let mut changes_made = false;

        for todo in &todos {
            if processed_repos.contains(&todo.repo) {
                all_done = false;
                continue;
            }
            processed_repos.insert(todo.repo.clone());

            let sp = ProgressBar::new_spinner();
            sp.set_style(spinner_style());
            sp.set_message(format!("{} MR !{}", todo.repo.bold(), todo.iid.cyan()));
            sp.enable_steady_tick(Duration::from_millis(80));

            let mr_info_output = Command::new("glab")
                .args(["mr", "view", &todo.iid, "-F", "json", "-R", &todo.repo])
                .output()
                .await;

            let mut issue_to_log: Option<String> = None;
            let mut mark_done = false;

            match mr_info_output {
                Ok(output) if output.status.success() => {
                    let mut mr_parsed = false;
                    if let Ok(mr_info) = serde_json::from_slice::<Value>(&output.stdout) {
                        mr_parsed = true;
                        let detailed_status =
                            mr_info["detailed_merge_status"].as_str().unwrap_or("");
                        let url = mr_info["web_url"].as_str().unwrap_or("");
                        let source_branch = mr_info["source_branch"].as_str().unwrap_or("");

                        sp.set_message(format!(
                            "{} MR !{} ({})",
                            todo.repo.bold(),
                            todo.iid.cyan(),
                            url.bright_black()
                        ));
                        all_done = false;

                        if detailed_status == "need_rebase" {
                            sp.set_message(format!(
                                "{} MR !{} - {}",
                                todo.repo.bold(),
                                todo.iid.cyan(),
                                "Rebasing...".yellow()
                            ));
                            let rebase_output = Command::new("glab")
                                .args(["mr", "rebase", &todo.iid, "-R", &todo.repo])
                                .output()
                                .await;
                            if rebase_output.map(|o| o.status.success()).unwrap_or(false) {
                                sp.finish_with_message(format!(
                                    "{} MR !{} {} Rebased successfully",
                                    todo.repo.bold(),
                                    todo.iid.cyan(),
                                    "✓".green()
                                ));
                            } else {
                                sp.finish_with_message(format!(
                                    "{} MR !{} {} Rebase failed - needs manual fix",
                                    todo.repo.bold(),
                                    todo.iid.cyan(),
                                    "❌".red()
                                ));
                                issue_to_log = Some(format!(
                                    "❌ Rebase failed for MR {url} - needs manual fix"
                                ));
                            }
                        } else if detailed_status == "mergeable" {
                            sp.set_message(format!(
                                "{} MR !{} - {}",
                                todo.repo.bold(),
                                todo.iid.cyan(),
                                "Merging...".cyan()
                            ));
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
                                .await;
                            if merge_output.map(|o| o.status.success()).unwrap_or(false) {
                                sp.finish_with_message(format!(
                                    "{} MR !{} {} MERGED!",
                                    todo.repo.bold(),
                                    todo.iid.cyan(),
                                    "✓".green().bold()
                                ));
                                mark_done = true;
                            } else {
                                sp.finish_with_message(format!(
                                    "{} MR !{} {} Merge failed",
                                    todo.repo.bold(),
                                    todo.iid.cyan(),
                                    "❌".red()
                                ));
                                issue_to_log = Some(format!("❌ Merge failed for MR {url}"));
                            }
                        } else {
                            // Check pipeline status
                            let mut pipeline_status = String::new();
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
                                .await;
                            if let Ok(ci_out) = ci_get_output
                                && let Ok(ci_info) = serde_json::from_slice::<Value>(&ci_out.stdout)
                                {
                                    pipeline_status =
                                        ci_info["status"].as_str().unwrap_or("").to_string();
                                }

                            if pipeline_status == "manual" {
                                sp.set_message(format!(
                                    "{} MR !{} - {}",
                                    todo.repo.bold(),
                                    todo.iid.cyan(),
                                    "Triggering manual pipeline...".yellow()
                                ));
                                let mut triggered = false;

                                let ci_jobs_output = Command::new("glab")
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
                                    .await;

                                if let Ok(j_out) = ci_jobs_output
                                    && let Ok(ci_json) =
                                        serde_json::from_slice::<Value>(&j_out.stdout)
                                        && let Some(jobs) = ci_json["jobs"].as_array() {
                                            for job in jobs {
                                                let stage = job["stage"].as_str().unwrap_or("");
                                                let status = job["status"].as_str().unwrap_or("");
                                                let job_id = job["id"].as_u64();
                                                if stage == "build" && status == "manual"
                                                    && let Some(jid) = job_id {
                                                        let trigger_out = Command::new("glab")
                                                            .args([
                                                                "ci",
                                                                "trigger",
                                                                &jid.to_string(),
                                                                "-R",
                                                                &todo.repo,
                                                            ])
                                                            .output()
                                                            .await;
                                                        if trigger_out
                                                            .map(|o| o.status.success())
                                                            .unwrap_or(false)
                                                        {
                                                            sp.finish_with_message(format!(
                                                                "{} MR !{} {} Pipeline triggered",
                                                                todo.repo.bold(),
                                                                todo.iid.cyan(),
                                                                "✓".green()
                                                            ));
                                                            triggered = true;
                                                            break;
                                                        }
                                                    }
                                            }
                                        }

                                if !triggered {
                                    sp.finish_with_message(format!(
                                        "{} MR !{} {} Manual action required",
                                        todo.repo.bold(),
                                        todo.iid.cyan(),
                                        "⚠".yellow()
                                    ));
                                    issue_to_log = Some(format!(
                                        "❌ Check what pipeline needs manual trigger for MR {url}"
                                    ));
                                }
                            } else if pipeline_status == "running" {
                                sp.finish_with_message(format!(
                                    "{} MR !{} {} Pipeline running...",
                                    todo.repo.bold(),
                                    todo.iid.cyan(),
                                    "⏳".bright_blue()
                                ));
                            } else if pipeline_status == "failed" {
                                sp.finish_with_message(format!(
                                    "{} MR !{} {} Pipeline failed",
                                    todo.repo.bold(),
                                    todo.iid.cyan(),
                                    "❌".red()
                                ));
                                issue_to_log = Some(format!(
                                    "❌ Pipeline failed for MR {url} - needs manual fix"
                                ));
                            } else {
                                match detailed_status {
                                    "not_approved" => sp.finish_with_message(format!(
                                        "{} MR !{} {} Waiting for approval",
                                        todo.repo.bold(),
                                        todo.iid.cyan(),
                                        "⏳".bright_blue()
                                    )),
                                    "ci_must_pass" | "ci_still_running" => {
                                        sp.finish_with_message(format!(
                                            "{} MR !{} {} Waiting for CI",
                                            todo.repo.bold(),
                                            todo.iid.cyan(),
                                            "⏳".bright_blue()
                                        ));
                                    }
                                    _ => sp.finish_with_message(format!(
                                        "{} MR !{} {} Status: {}",
                                        todo.repo.bold(),
                                        todo.iid.cyan(),
                                        "⏳".bright_blue(),
                                        detailed_status.bright_black()
                                    )),
                                }
                            }
                        }
                    }
                    if !mr_parsed {
                        sp.finish_with_message(format!(
                            "{} MR !{} {} Failed to parse MR info",
                            todo.repo.bold(),
                            todo.iid.cyan(),
                            "❌".red()
                        ));
                    }
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    sp.finish_with_message(format!(
                        "{} MR !{} {} glab error: {}",
                        todo.repo.bold(),
                        todo.iid.cyan(),
                        "❌".red(),
                        stderr.trim()
                    ));
                    issue_to_log =
                        Some(format!("❌ ERROR: glab request failed: {}", stderr.trim()));
                    all_done = false;
                }
                Err(e) => {
                    sp.finish_with_message(format!(
                        "{} MR !{} {} glab error: {}",
                        todo.repo.bold(),
                        todo.iid.cyan(),
                        "❌".red(),
                        e
                    ));
                    issue_to_log = Some(format!(
                        "❌ ERROR: Cannot access/run glab for {}",
                        todo.repo
                    ));
                    all_done = false;
                }
            }

            if let Ok(refreshed_content) = tokio::fs::read_to_string(&org_file).await {
                lines = refreshed_content.lines().map(String::from).collect();
            }

            let mut target_idx = None;
            for (i, line) in lines.iter().enumerate() {
                if let Some(caps) = re_todo.captures(line) {
                    let c_repo = caps[2].to_string();
                    let c_iid = caps[3].to_string();
                    if c_repo == todo.repo && c_iid == todo.iid {
                        target_idx = Some(i);
                        break;
                    }
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
                        changes_made = true;
                    } else {
                        i += 1;
                    }
                }

                if mark_done {
                    let current_line = &mut lines[start_idx];
                    *current_line = current_line.replacen("TODO", ("DONE".to_string()).as_str(), 1);
                    changes_made = true;
                } else if let Some(msg) = issue_to_log {
                    lines.insert(start_idx + 1, msg);
                    changes_made = true;
                }

                if changes_made {
                    let _ = tokio::fs::write(&org_file, lines.join("\n") + "\n").await;
                    changes_made = false;

                    tokio::time::sleep(Duration::from_millis(100)).await;
                    while rx.try_recv().is_ok() {}
                }
            }
        }

        if all_done && !todos.is_empty() {
            println!(
                "\n  {} {}",
                "✓".green().bold(),
                "ALL REPOSITORIES IN QUEUE CHECKED".bold()
            );
        }

        println!(
            "\n  {} Next check in {} seconds (or when file changes)...",
            "⏱".cyan(),
            interval.as_secs()
        );
        wait_for_event(&rx, interval).await;
    }
}

async fn wait_for_event(
    rx: &mpsc::Receiver<Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>>,
    default_interval: Duration,
) {
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() >= default_interval {
            break;
        }

        let mut got_event = false;
        while rx.try_recv().is_ok() {
            got_event = true;
        }

        if got_event {
            break;
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}
