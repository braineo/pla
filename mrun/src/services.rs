use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use owo_colors::OwoColorize;
use owo_colors::colors::*;

use crate::cli::Args;

#[derive(Debug, Clone)]
struct Repository {
    name: String,
    path: PathBuf,
}

impl Repository {
    fn display_name(&self) -> String {
        format!("{} ({})", self.name.bright_cyan(), self.path.display())
    }

    fn is_git_repo(&self) -> bool {
        self.path.join(".git").exists()
    }

    fn get_status(&self) -> Result<String> {
        let output = Command::new("git")
            .args(["status", "--short"])
            .current_dir(&self.path)
            .output()
            .context("Failed to get git status")?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

fn walk_repositories(root: &Path, pattern: Option<&str>) -> Vec<Repository> {
    let mut repos = Vec::new();

    let pattern_reg = pattern.map(|p| regex::Regex::new(p).expect("Invalid regex pattern"));

    if let Ok(paths) = fs::read_dir(root) {
        paths
            .filter_map(|p| {
                let entry = p.ok()?;
                let path = entry.path();
                let name = path.file_name()?.to_str()?.to_string();

                if !(path.is_dir() && path.join(".git").exists()) {
                    return None;
                }

                if reg.is_match(&name) {
                    return Some(Repository { name, path });
                } else {
                    return None;
                }
            })
            .collect_into(&mut repos);
    }

    repos
}

pub async fn run(args: Args) -> Result<()> {
    Ok(())
}
