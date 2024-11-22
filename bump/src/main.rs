use crate::{repo::Repo, settings::Settings};
use anyhow::bail;
use clap::{value_parser, Arg, ArgAction, Command, ValueEnum};
use clap_complete::{generate, Generator, Shell};
use config::Config;
use log::{debug, info};
use owo_colors::{colors::xterm, OwoColorize};
use semver::Version;
use serde::{Deserialize, Serialize};

use std::{
    env,
    fs::File,
    io,
    path::{Path, PathBuf},
};

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
    /// Add identifiers to version for prerelease
    fn append_prerelease_identifiers(&self, identifiers: &str) -> Self;
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
        let next_pre = semver::Prerelease::new(&next_pre).expect("pre release increment failed.");
        Self {
            pre: next_pre,
            ..self.clone()
        }
    }

    fn append_prerelease_identifiers(&self, identifiers: &str) -> Self {
        let next_pre = semver::Prerelease::new(identifiers).expect("pre release increment failed.");
        Self {
            pre: next_pre,
            ..self.clone()
        }
    }
}

fn increment_last_identifier(release: &str) -> String {
    if let Ok(release_number) = release.parse::<u32>() {
        return (release_number + 1).to_string();
    }
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
                .value_name("BUMP_TYPE")
                .help("which version to bump to")
                .value_parser(value_parser!(BumpType)),
        )
        .arg(
            Arg::new("project_path")
                .long("path")
                .value_name("PATH")
                .help("the directory to execute bump")
                .required(false)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("prerelease")
                .long("prerelease")
                .value_name("IDENTIFIER")
                .help("specify a IDENTIFIER for prerelesae, prerelease version will be -IDENTIFIER.0 or -0")
                .required(false)
                .num_args(0..=1)
                .default_missing_value("")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("dryrun")
                .long("dryrun")
                .help("skip file change and commit")
                .action(clap::ArgAction::SetTrue),
        )
        .subcommand(
            Command::new("completions").arg(
                Arg::new("shell")
                    .long("shell")
                    .action(ArgAction::Set)
                    .value_parser(value_parser!(Shell)),
            ),
        )
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_default_env().init();

    let matches = cli().get_matches();

    if let Some(("completions", completions_matches)) = matches.subcommand() {
        if let Some(shell) = completions_matches.get_one::<Shell>("shell").copied() {
            let mut cmd = cli();

            print_completions(shell, &mut cmd);
        } else {
            eprintln!("cannot generate auto completions");
        }
        return Ok(());
    }

    let project_repo = if let Some(project_path) = matches.get_one::<PathBuf>("project_path") {
        Repo::new(project_path.clone())?
    } else {
        Repo::new(env::current_dir()?)?
    };

    let settings: Settings = Config::builder()
        .add_source(config::File::from(project_repo.directory.join("bump")))
        .build()?
        .try_deserialize::<Settings>()?;

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

    let prerelease = matches.get_one::<String>("prerelease");
    info!("prerelease {:?}", prerelease);

    let next_version = if let Some(bump_type) = matches.get_one::<BumpType>("bump_type") {
        match bump_type {
            BumpType::Major => version.increment_major().to_string(),
            BumpType::Minor => version.increment_minor().to_string(),
            BumpType::Patch => version.increment_patch().to_string(),
        }
    } else if let Some(prerelease) = matches.get_one::<String>("prerelease") {
        if version.pre.is_empty() {
            if prerelease.is_empty() {
                version.append_prerelease_identifiers("0").to_string()
            } else {
                version
                    .append_prerelease_identifiers(&format!("{prerelease}.0"))
                    .to_string()
            }
        } else if !prerelease.is_empty() {
            bail!(
                "prerelease identifiers exists {} but got specified as {}",
                version.pre,
                prerelease
            );
        } else {
            version.increment_prerelease().to_string()
        }
    } else {
        bail!("need to bump to at lease one of major, minor or patch")
    };

    if matches.get_flag("dryrun") {
        println!(
            "{} {}{}",
            "will bump version to".bg::<xterm::Gray>(),
            settings.tag_prefix.green(),
            next_version.green()
        );

        println!(
            "{} {}",
            "will bump files".bg::<xterm::Gray>(),
            std::iter::once(package_json_file_name.to_string())
                .chain(settings.bump_files.into_iter())
                .collect::<Vec<_>>()
                .join(", ")
                .green()
        );
        return Ok(());
    }

    info!("bump to version {}", next_version);
    project_repo.bump_package_json(package_json_file_name, &next_version)?;
    project_repo.stage_file(package_json_file_name)?;

    debug!("bump other files {:?}", settings.bump_files);

    for bump_file in settings.bump_files {
        if !Path::new(&bump_file).exists() {
            debug!("{bump_file} does not exist, skip.");
            continue;
        }

        project_repo.bump_package_json(&bump_file, &next_version)?;
        project_repo.stage_file(&bump_file)?;
    }

    project_repo.commit_and_tag_release(&next_version, &settings.tag_prefix)?;

    Ok(())
}
