use anyhow::bail;
use clap::{value_parser, Arg, Command, ValueEnum};
use config::Config;
use log::{debug, info};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{env, fs::File, path::PathBuf};

use crate::{repo::Repo, settings::Settings};

pub mod repo;
pub mod settings;

pub trait Bump {
    /// Increments the major version number.
    fn increment_major(&self) -> Self;
    /// Increments the minor version number.
    fn increment_minor(&self) -> Self;
    /// Increments the patch version number.
    fn increment_patch(&self) -> Self;
    /// Increments the prerelease version number.
    fn increment_prerelease(&self) -> Self;
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, ValueEnum)]
pub enum BumpType {
    /// Bump major version.
    Major,
    /// Bump minor version.
    Minor,
    /// Bump patch version.
    Patch,
}

impl Bump for Version {
    // taken from https://github.com/killercup/cargo-edit/blob/643e9253a84db02c52a7fa94f07d786d281362ab/src/version.rs#L38
    fn increment_major(&self) -> Self {
        Self {
            major: self.major + 1,
            minor: 0,
            patch: 0,
            pre: semver::Prerelease::EMPTY,
            build: self.build.clone(),
        }
    }

    // taken from https://github.com/killercup/cargo-edit/blob/643e9253a84db02c52a7fa94f07d786d281362ab/src/version.rs#L46
    fn increment_minor(&self) -> Self {
        Self {
            minor: self.minor + 1,
            patch: 0,
            pre: semver::Prerelease::EMPTY,
            ..self.clone()
        }
    }

    // taken from https://github.com/killercup/cargo-edit/blob/643e9253a84db02c52a7fa94f07d786d281362ab/src/version.rs#L53
    fn increment_patch(&self) -> Self {
        Self {
            patch: self.patch + 1,
            pre: semver::Prerelease::EMPTY,
            ..self.clone()
        }
    }

    fn increment_prerelease(&self) -> Self {
        let next_pre = increment_last_identifier(self.pre.as_str());
        let next_pre = semver::Prerelease::new(&next_pre).expect("pre release increment failed. Please report this issue to https://github.com/MarcoIeni/release-plz/issues");
        Self {
            pre: next_pre,
            ..self.clone()
        }
    }
}

fn increment_last_identifier(release: &str) -> String {
    match release.rsplit_once('.') {
        Some((left, right)) => {
            if let Ok(right_num) = right.parse::<u32>() {
                format!("{left}.{}", right_num + 1)
            } else {
                format!("{release}.1")
            }
        }
        None => format!("{release}.1"),
    }
}

fn cli() -> Command {
    Command::new("bump")
        .about("bump version in package json, and tag commit")
        .arg(
            Arg::new("bump_type")
                .long("type")
                .value_parser(value_parser!(BumpType)),
        )
        .arg(
            Arg::new("project_path")
                .long("path")
                .required(false)
                .value_parser(value_parser!(PathBuf)),
        )
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_default_env().init();

    let matches = cli().get_matches();

    let project_repo = if let Some(project_path) = matches.get_one::<PathBuf>("project_path") {
        Repo::new(project_path.clone())?
    } else {
        Repo::new(env::current_dir()?)?
    };

    let default_settings = Settings {
        bump_files: Some(vec![String::from("package-lock.json")]),
    };
    let settings = match Config::builder()
        .add_source(config::File::from(project_repo.directory.join("bump")))
        .build()
    {
        Ok(config_builder) => config_builder
            .try_deserialize::<Settings>()
            .unwrap_or(default_settings),
        _ => default_settings,
    };

    let package_json_file_name = "package.json";

    let package_json_file = File::open(project_repo.directory.join(package_json_file_name))?;
    let package_json: serde_json::Value = serde_json::from_reader(package_json_file)?;

    let version = if let Some(version_value) = package_json.get("version") {
        let version_str = version_value
            .as_str()
            .expect("it should be able to convert to str");
        Version::parse(version_str)?
    } else {
        bail!("cannot find version in package.json");
    };

    let next_version = if let Some(bump_type) = matches.get_one::<BumpType>("bump_type") {
        match bump_type {
            BumpType::Major => version.increment_major().to_string(),
            BumpType::Minor => version.increment_minor().to_string(),
            BumpType::Patch => version.increment_patch().to_string(),
        }
    } else {
        bail!("need to bump to at lease one of major, minor or patch")
    };

    info!("bump to version {}", next_version);
    project_repo.bump_package_json(package_json_file_name, &next_version)?;
    project_repo.stage_file(package_json_file_name)?;

    debug!("bump other files {:?}", settings.bump_files);
    if let Some(bump_files) = settings.bump_files {
        for bump_file in bump_files {
            project_repo.bump_package_json(&bump_file, &next_version)?;
            project_repo.stage_file(&bump_file)?;
        }
    }

    project_repo.commit_and_tag_release(&next_version)?;

    Ok(())
}
