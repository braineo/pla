use std::path::Path;

use config::Config;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct RawSettings {
    pub version_file: Option<String>,
    pub bump_files: Option<Vec<String>>,
    pub tag_prefix: Option<String>,
}

#[derive(Debug)]
pub struct Settings {
    pub version_file: String,
    pub bump_files: Vec<String>,
    pub tag_prefix: String,
}

const CONFIG_FILE_NAME: &str = "bump";

pub fn init_settings(project_path: &Path) -> anyhow::Result<Settings> {
    let raw_settings = Config::builder()
        .add_source(config::File::from(project_path.join(CONFIG_FILE_NAME)).required(false))
        .build()?
        .try_deserialize::<RawSettings>()?;

    let tag_prefix = raw_settings.tag_prefix.unwrap_or_else(|| "v".to_string());

    let version_file = match raw_settings.version_file {
        Some(version_file) => version_file,
        None => {
            let candidates = vec!["package.json", "Cargo.toml"];

            candidates
                .into_iter()
                .find_map(|file_candidate| {
                    if project_path.join(file_candidate).exists() {
                        Some(file_candidate.to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| "package.json".to_string())
        }
    };

    let bump_files = match raw_settings.bump_files {
        Some(files) => files,
        None => generate_default_bump_files(&version_file, project_path),
    };

    Ok(Settings {
        version_file,
        bump_files,
        tag_prefix,
    })
}

fn generate_default_bump_files(version_file: &str, project_path: &Path) -> Vec<String> {
    let mut bump_files = Vec::new();

    // Add additional files based on the version file type
    match version_file {
        "package.json" => {
            // For Node.js projects, include package-lock.json
            let package_lock = "package-lock.json";
            if project_path.join(package_lock).exists() {
                bump_files.push(package_lock.to_string());
            }
        }
        "Cargo.toml" => {
            // For Rust projects, include Cargo.lock
            let cargo_lock = "Cargo.lock";
            if project_path.join(cargo_lock).exists() {
                bump_files.push(cargo_lock.to_string());
            }
        }
        // Add more cases as needed for other project types
        _ => {
            // For unknown version file types, just include the version file itself
        }
    }

    bump_files
}
