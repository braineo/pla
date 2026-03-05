use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use regex::Regex;
use serde::Deserialize;
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{process::Command, sync::mpsc};

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

#[derive(Deserialize, Debug)]
struct MrViewResponse {
    detailed_merge_status: Option<String>,
    web_url: Option<String>,
    source_branch: Option<String>,
    title: Option<String>,
}

#[derive(Deserialize, Debug)]
struct CiPipelineResponse {
    status: Option<String>,
}

#[derive(Deserialize, Debug)]
struct CiJobResponse {
    jobs: Option<Vec<CiJob>>,
}

#[derive(Deserialize, Debug)]
struct CiJob {
    id: Option<u64>,
    stage: Option<String>,
    status: Option<String>,
}

#[derive(Clone, Debug)]
struct MrState {
    repo: String,
    iid: String,
    title: String,
    url: String,
    status_text: String,
    started_at: Instant,
    done: bool,
    completed_in: Option<Duration>,
}

enum AppEvent {
    Tick,
    FileChanged,
    UiEvent(Event),
    MrStateUpdate {
        repo: String,
        iid: String,
        title: Option<String>,
        url: Option<String>,
        status_text: Option<String>,
        done: Option<bool>,
    },
    MarkDoneInFile {
        repo: String,
        iid: String,
    },
    LogIssueInFile {
        repo: String,
        iid: String,
        message: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // We don't initialize env_logger to avoid scrambling the TUI

    let args = Args::parse();
    let org_file = if args.org_file.is_absolute() {
        args.org_file.clone()
    } else {
        std::env::current_dir()?.join(&args.org_file)
    };
    let org_file = Arc::new(org_file);

    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();

    let tx_file = tx.clone();
    let mut debouncer_opt = new_debouncer(
        Duration::from_secs(1),
        move |res: Result<Vec<notify_debouncer_mini::DebouncedEvent>, _>| {
            if res.is_ok() {
                let _ = tx_file.send(AppEvent::FileChanged);
            }
        },
    )
    .ok();

    if org_file.exists()
        && let Some(parent) = org_file.parent()
        && let Some(ref mut debouncer) = debouncer_opt
    {
        let _ = debouncer
            .watcher()
            .watch(parent, RecursiveMode::NonRecursive);
    }

    let tx_ui = tx.clone();
    std::thread::spawn(move || {
        loop {
            if let Ok(true) = event::poll(Duration::from_millis(250)) {
                if let Ok(e) = event::read()
                    && tx_ui.send(AppEvent::UiEvent(e)).is_err()
                {
                    break;
                }
            } else if tx_ui.send(AppEvent::Tick).is_err() {
                break;
            }
        }
    });

    let interval = args.interval;
    let tx_poll = tx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(interval)).await;
            let _ = tx_poll.send(AppEvent::FileChanged);
        }
    });

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut mr_list: Vec<MrState> = Vec::new();
    let mut active_tasks: HashSet<String> = HashSet::new();

    let _ = tx.send(AppEvent::FileChanged);

    loop {
        terminal.draw(|f| draw_ui(f, &mr_list))?;

        if let Some(event) = rx.recv().await {
            match event {
                AppEvent::UiEvent(Event::Key(key)) => {
                    if key.code == KeyCode::Char('q')
                        || key.code == KeyCode::Esc
                        || (key.code == KeyCode::Char('c')
                            && key.modifiers.contains(event::KeyModifiers::CONTROL))
                    {
                        break;
                    }
                }
                AppEvent::Tick => {}
                AppEvent::FileChanged => {
                    if let Ok(todos) = read_todos(&org_file).await {
                        for todo in todos {
                            let key = format!("{}!{}", todo.repo, todo.iid);
                            if !active_tasks.contains(&key) {
                                active_tasks.insert(key.clone());
                                mr_list.push(MrState {
                                    repo: todo.repo.clone(),
                                    iid: todo.iid.clone(),
                                    title: String::new(),
                                    url: String::new(),
                                    status_text: "Initializing...".to_string(),
                                    started_at: Instant::now(),
                                    done: false,
                                    completed_in: None,
                                });
                                spawn_mr_processor(todo, tx.clone());
                            }
                        }
                    }
                }
                AppEvent::MrStateUpdate {
                    repo,
                    iid,
                    title,
                    url,
                    status_text,
                    done,
                } => {
                    if let Some(mr) = mr_list.iter_mut().find(|m| m.repo == repo && m.iid == iid) {
                        if let Some(t) = title {
                            mr.title = t;
                        }
                        if let Some(u) = url {
                            mr.url = u;
                        }
                        if let Some(s) = status_text {
                            mr.status_text = s;
                        }
                        if let Some(d) = done {
                            mr.done = d;
                            if d && mr.completed_in.is_none() {
                                mr.completed_in = Some(mr.started_at.elapsed());
                            }
                        }
                    }
                }
                AppEvent::MarkDoneInFile { repo, iid } => {
                    mark_done_in_file(&org_file, &repo, &iid).await;
                }
                AppEvent::LogIssueInFile { repo, iid, message } => {
                    log_issue_in_file(&org_file, &repo, &iid, &message).await;
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

async fn read_todos(org_file: &std::path::Path) -> Result<Vec<TodoItem>> {
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

fn spawn_mr_processor(todo: TodoItem, tx: mpsc::UnboundedSender<AppEvent>) {
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

async fn process_mr_once(todo: &TodoItem, tx: &mpsc::UnboundedSender<AppEvent>) -> Result<bool> {
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
        return Ok(false);
    } else if detailed_status == "mergeable" {
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
            return Ok(true);
        } else {
            let _ = tx.send(AppEvent::LogIssueInFile {
                repo: todo.repo.clone(),
                iid: todo.iid.clone(),
                message: format!("❌ Merge failed for MR {url}"),
            });
            anyhow::bail!("Merge failed");
        }
    } else {
        let ci_get_output = Command::new("glab")
            .args([
                "ci",
                "get",
                "--branch",
                &source_branch,
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
                        &source_branch,
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
                        message: format!(
                            "❌ Check what pipeline needs manual trigger for MR {url}"
                        ),
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
    }

    Ok(false)
}

async fn mark_done_in_file(org_file: &std::path::Path, repo: &str, iid: &str) {
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

async fn log_issue_in_file(org_file: &std::path::Path, repo: &str, iid: &str, message: &str) {
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

fn draw_ui(f: &mut Frame, mr_list: &[MrState]) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0)])
        .split(f.area());

    if mr_list.is_empty() {
        let empty_msg = Paragraph::new("No MRs found in tracking file. You can add more.")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(" wum - MR Watcher (Press 'q' to quit) ")
                    .borders(Borders::ALL),
            );
        f.render_widget(empty_msg, chunks[0]);
        return;
    }

    let mut items = Vec::new();
    for mr in mr_list {
        let elapsed = mr.completed_in.unwrap_or_else(|| mr.started_at.elapsed());
        let secs = elapsed.as_secs() % 60;
        let mins = (elapsed.as_secs() / 60) % 60;
        let hours = elapsed.as_secs() / 3600;
        let time_str = if hours > 0 {
            format!("{hours:02}:{mins:02}:{secs:02}")
        } else {
            format!("{mins:02}:{secs:02}")
        };

        let status_color = if mr.done {
            Color::Green
        } else if mr.status_text.contains("Error")
            || mr.status_text.contains("failed")
            || mr.status_text.contains("Manual")
        {
            Color::Red
        } else if mr.status_text.contains("Rebasing") {
            Color::Yellow
        } else {
            Color::Cyan
        };

        let state_line = Line::from(vec![
            Span::styled(
                format!("{} ", mr.status_text),
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{}!{}", mr.repo, mr.iid),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" <"),
            Span::styled(time_str, Style::default().fg(Color::DarkGray)),
            Span::raw(">"),
        ]);

        let title_line = Line::from(vec![Span::styled(
            &mr.title,
            Style::default().fg(Color::White),
        )]);

        let url_line = Line::from(vec![Span::styled(
            &mr.url,
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::UNDERLINED),
        )]);

        items.push(ListItem::new(vec![
            state_line,
            title_line,
            url_line,
            Line::raw(""),
        ]));
    }

    let list = List::new(items).block(
        Block::default()
            .title(" wum - MR Watcher (Press 'q' to quit) ")
            .borders(Borders::ALL),
    );

    f.render_widget(list, chunks[0]);
}
