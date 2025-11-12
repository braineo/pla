use anyhow::{Context, Result};
use inquire::Confirm;
use inquire::MultiSelect;
use owo_colors::OwoColorize;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;
use std::process::Stdio;

use crate::cli::Args;
use crate::settings;
use crate::settings::write_settings;

#[derive(Debug, Clone)]
struct Repository {
    name: String,
    path: PathBuf,
}

impl Repository {
    fn get_status(&self) -> Result<String> {
        let output = Command::new("git")
            .args(["status", "--short"])
            .current_dir(&self.path)
            .output()
            .context("Failed to get git status")?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub fn run_command<I, K, V>(&self, command: &str, vars: I) -> Result<Output>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        Command::new("bash")
            .arg("-c")
            .arg(command)
            .envs(vars)
            .current_dir(&self.path)
            .stdout(Stdio::inherit())
            .output()
            .context("Failed to execute command in {self.name}")
    }
}

impl std::fmt::Display for Repository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, ({})", self.name.bright_cyan(), self.path.display(),)
    }
}

fn walk_repositories(root: &Path, pattern: Option<&str>) -> Vec<Repository> {
    let mut repos = Vec::new();

    let pattern_reg = pattern.map(|p| regex::Regex::new(p).expect("Invalid regex pattern"));

    if let Ok(paths) = fs::read_dir(root) {
        repos.extend(paths.filter_map(|p| -> Option<Repository> {
            let entry = p.ok()?;
            let path = entry.path();
            let name = path.file_name()?.to_str()?.to_string();

            if !(path.is_dir() && path.join(".git").exists()) {
                return None;
            }

            if let Some(reg) = &pattern_reg
                && !reg.is_match(&name)
            {
                return None;
            }

            Some(Repository { name, path })
        }));
    }

    repos
}

fn run_ls_command(root: &Path, command: &str, pattern: Option<&str>) -> Vec<Repository> {
    let mut repos = Vec::new();

    let pattern_reg = pattern.map(|p| regex::Regex::new(p).expect("Invalid regex pattern"));

    let command_result = Command::new("bash")
        .arg("-c")
        .arg(command)
        .current_dir(root)
        .output()
        .context("Failed to execute command in {self.name}");

    if let Ok(output) = command_result {
        repos.extend(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|name| {
                    if let Some(reg) = &pattern_reg
                        && !reg.is_match(name)
                    {
                        return None;
                    }
                    Some(Repository {
                        name: name.to_string(),
                        path: root.join(name),
                    })
                }),
        );
    }

    repos
}

fn select_repositories(
    repos: Vec<Repository>,
    default_selection: &[String],
) -> Result<Vec<Repository>> {
    if repos.is_empty() {
        anyhow::bail!("No repositories found!");
    }

    println!("\n{}", "Found repositories:".bright_green().bold());

    let (mut selected_repos, mut unselected_repos): (Vec<_>, Vec<_>) = repos
        .into_iter()
        .partition(|repo| default_selection.contains(&repo.name));

    selected_repos.append(&mut unselected_repos);
    let repos = selected_repos;

    let default: Vec<_> = (0..default_selection.len().min(repos.len())).collect();

    let selected = MultiSelect::new(
        "Select repositories to process (Space to select, Enter to confirm):",
        repos,
    )
    .with_default(&default)
    .with_page_size(20)
    .prompt()
    .context("Failed to get repository selection")?;

    Ok(selected)
}

fn get_command(
    command_string: Option<String>,
    command_file_path: Option<PathBuf>,
) -> Result<String> {
    if let Some(cmd) = command_string {
        return Ok(cmd);
    }

    if let Some(path) = command_file_path {
        return Ok(fs::read_to_string(path)?);
    }

    let command = inquire::Text::new("Enter command to execute in each repository:")
        .with_placeholder("e.g., git pull && npm install")
        .with_help_message("This will be executed as: bash -c 'your command'")
        .prompt()
        .context("Failed to get command")?;

    Ok(command)
}

fn batch_run(repos: &[Repository], command: &str) -> Result<HashMap<String, bool>> {
    let mut results = HashMap::new();
    let mut index = 1;

    for repo in repos {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("{} {index} / {}\n", repo.name, repos.len());
        let output = repo.run_command(command, [("REPO_NAME", repo.name.as_str())])?;
        results.insert(repo.name.clone(), output.status.success());
        index += 1;
        println!("\n");
    }
    Ok(results)
}

pub async fn run(args: Args) -> Result<()> {
    let mut settings = settings::load_settings().context("Failed to load settings")?;

    let repos = if let Some(list_command) = args.list_command {
        run_ls_command(&args.dir, &list_command, args.match_regexp.as_deref())
    } else {
        walk_repositories(&args.dir, args.match_regexp.as_deref())
    };

    if repos.is_empty() {
        println!("{} No repositories found!", "⚠".yellow());
        return Ok(());
    }

    let selected_repos = select_repositories(
        repos,
        if let Some(resume_failed) = args.failed
            && resume_failed
        {
            &settings.last_failed_repos
        } else {
            &settings.last_selected_repos
        },
    )?;

    if selected_repos.is_empty() {
        println!("\n{} No repositories selected. Exiting.", "ℹ".blue());
        return Ok(());
    }

    settings.last_selected_repos = selected_repos
        .iter()
        .map(|repo| repo.name.clone())
        .collect();

    write_settings(&settings)?;

    println!(
        "\n{} Selected {} repositories",
        "✓".bright_green(),
        selected_repos.len().to_string().bright_cyan()
    );

    let command = get_command(args.command, args.command_file)?;

    println!("\n{}", "Command to execute:".bright_yellow());
    println!("  {}\n", command.bright_white());

    // Confirm execution
    let confirm = Confirm::new("Proceed with batch execution?")
        .with_default(false)
        .prompt()
        .context("Failed to get confirmation")?;

    if !confirm {
        println!("\n{} Execution cancelled.", "✗".yellow());
        return Ok(());
    }

    let results = batch_run(&selected_repos, &command)?;

    let mut failed_repos = Vec::new();
    for (name, success) in results.into_iter() {
        if success {
            println!("{} {}", "✓".bright_green(), name.as_str().bright_cyan(),);
        } else {
            failed_repos.push(name.clone());
            println!("{} {}", "✗".bright_red(), name.as_str().bright_cyan(),);
        }
    }

    settings.last_failed_repos = failed_repos;
    write_settings(&settings)?;

    Ok(())
}
