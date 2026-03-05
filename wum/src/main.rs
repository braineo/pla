mod gitlab;
mod org;
mod types;
mod ui;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use ratatui::{Terminal, prelude::*};
use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::mpsc;

use crate::gitlab::spawn_mr_processor;
use crate::org::{log_issue_in_file, mark_done_in_file, read_todos};
use crate::types::{AppEvent, Args, MrState};
use crate::ui::draw_ui;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let org_file = if args.org_file.is_absolute() {
        args.org_file.clone()
    } else {
        std::env::current_dir()?.join(&args.org_file)
    };
    let org_file = Arc::new(org_file);

    let (tx, rx) = mpsc::unbounded_channel::<AppEvent>();

    let _debouncer = setup_file_watcher(&org_file, tx.clone());
    setup_ui_event_loop(tx.clone());

    let interval = args.interval;
    let tx_poll = tx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(interval)).await;
            let _ = tx_poll.send(AppEvent::FileChanged);
        }
    });

    run_app(tx, rx, org_file.clone()).await
}

fn setup_file_watcher(
    org_file: &Arc<std::path::PathBuf>,
    tx: mpsc::UnboundedSender<AppEvent>,
) -> Option<notify_debouncer_mini::Debouncer<notify_debouncer_mini::notify::RecommendedWatcher>> {
    let tx_file = tx;
    let debouncer_opt = new_debouncer(
        Duration::from_secs(1),
        move |res: Result<Vec<notify_debouncer_mini::DebouncedEvent>, _>| {
            if res.is_ok() {
                let _ = tx_file.send(AppEvent::FileChanged);
            }
        },
    );

    if let Ok(mut debouncer) = debouncer_opt
        && org_file.exists()
        && let Some(parent) = org_file.parent()
    {
        let _ = debouncer
            .watcher()
            .watch(parent, RecursiveMode::NonRecursive);
        return Some(debouncer);
    }
    None
}

fn setup_ui_event_loop(tx_ui: mpsc::UnboundedSender<AppEvent>) {
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
}

async fn run_app(
    tx: mpsc::UnboundedSender<AppEvent>,
    mut rx: mpsc::UnboundedReceiver<AppEvent>,
    org_file: Arc<std::path::PathBuf>,
) -> Result<()> {
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
                AppEvent::UiEvent(_) | AppEvent::Tick => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
