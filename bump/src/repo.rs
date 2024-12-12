use anyhow::{anyhow, Context};
use log::info;
use serde_json::json;
use std::{fs::File, io::Write, path::PathBuf, process};

#[derive(Debug, Clone)]
pub struct Repo {
    pub directory: PathBuf,
}

impl Repo {
    pub fn new(directory: PathBuf) -> anyhow::Result<Self> {
        info!("create repo struct in {}", directory.to_string_lossy());
        if directory.exists() {
            Ok(Self { directory })
        } else {
            Err(anyhow!("{} does not exists.", directory.to_string_lossy()))
        }
    }

    pub fn stage_file(&self, file_name: &str) -> anyhow::Result<String> {
        run_git_command(&self.directory, &["add", file_name])
    }

    pub fn commit_changes(&self, next_version: &str) -> anyhow::Result<String> {
        let message = format!("chore(release): {next_version}");
        run_git_command(&self.directory, &["commit", "-m", &message])?;

        Ok(String::from(""))
    }

    pub fn tag_release(&self, next_version: &str, tag_prefix: &str) -> anyhow::Result<String> {
        let message = format!("chore(release): {next_version}");
        run_git_command(
            &self.directory,
            &[
                "tag",
                "-a",
                &format!("{tag_prefix}{next_version}"),
                "-m",
                &message,
            ],
        )?;

        Ok(String::from(""))
    }

    pub fn bump_json(&self, file_path: &str, next_version: &str) -> anyhow::Result<()> {
        info!("bump {} to {}", file_path, next_version);
        let full_path = self.directory.join(file_path);
        let json_file = File::open(&full_path)?;
        let mut package_json: serde_json::Value = serde_json::from_reader(json_file)?;

        if let Some(version) = package_json.get_mut("version") {
            *version = json!(next_version);
        }

        let mut file = File::create(&full_path)?;
        let updated_package_json_str = serde_json::to_string_pretty(&package_json)?;

        file.write_all(updated_package_json_str.as_bytes())?;

        Ok(())
    }
}

fn run_git_command(dir: &PathBuf, args: &[&str]) -> anyhow::Result<String> {
    let args: Vec<&str> = args.iter().map(|s| s.trim()).collect();
    let output = process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(&args)
        .output()
        .with_context(|| {
            format!("error while running git in directory `{dir:?}` with args `{args:?}`")
        })?;

    info!("git {:?}: output = {:?}", args, output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    if output.status.success() {
        Ok(stdout.as_ref().into())
    } else {
        let mut error =
            format!("error while running git in directory `{dir:?}` with args `{args:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !stdout.is_empty() || !stderr.is_empty() {
            error.push(':');
        }
        if !stdout.is_empty() {
            error.push_str("\n- stdout: ");
            error.push_str(&stdout);
        }
        if !stderr.is_empty() {
            error.push_str("\n- stderr: ");
            error.push_str(&stderr);
        }
        Err(anyhow!(error))
    }
}
