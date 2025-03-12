use crate::repo::Repo;
use anyhow::{anyhow, bail, Context, Result};
use bump_version::{BumpType, BumpVersion};
use clap::{value_parser, Arg, ArgAction, Command, ValueEnum};
use clap_complete::{generate, Generator, Shell};
use cli::prompt_version_select;

use log::{debug, info};
use owo_colors::{colors::xterm, OwoColorize};
use semver::Version;
use serde::{Deserialize, Serialize};
use settings::init_settings;
use toml_edit::DocumentMut;

use std::{
    env,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};

pub mod bump_version;
pub mod cli;
pub mod repo;
pub mod settings;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, ValueEnum, PartialOrd, Ord)]
pub enum Action {
    /// Make new commit for changes
    Commit,
    /// Tag the latest commit
    Tag,
}

fn cli() -> Command {
    Command::new("bump")
        .about("bump version in package.json or Cargo.toml, and tag commit")
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
            Arg::new("pre_id")
                .long("pre-id")
                .value_name("IDENTIFIER")
                .help(
                    "specify a IDENTIFIER for prerelesae, \
prerelease version will be -IDENTIFIER.0 or -0",
                )
                .required(false)
                .num_args(0..=1)
                .default_missing_value("")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("skip")
                .long("skip")
                .value_name("ACTION")
                .help("skip commit or tag")
                .action(clap::ArgAction::Append)
                .value_parser(value_parser!(Action)),
        )
        .arg(
            Arg::new("dryrun")
                .long("dryrun")
                .help("preview what will happen to the repo")
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

#[derive(Debug)]
enum VersionFileFormat {
    Json,
    Toml,
}

fn detect_file_format(file_path: &Path) -> Result<VersionFileFormat> {
    match file_path.extension().and_then(|ext| ext.to_str()) {
        Some("json") => Ok(VersionFileFormat::Json),
        Some("toml") => Ok(VersionFileFormat::Toml),
        _ => Err(anyhow!(
            "cannot determine file format for '{}', supported formats are JSON and TOML",
            file_path.display()
        )),
    }
}

fn get_version_from_file(file_path: &Path) -> Result<Version> {
    let file_name = match file_path.file_name() {
        Some(file_name) => file_name,
        _ => return Err(anyhow!("path does not contain file name")),
    };

    let format = detect_file_format(file_path)?;

    match format {
        VersionFileFormat::Json => {
            let file = File::open(file_path)
                .with_context(|| format!("Failed to open JSON file: {}", file_path.display()))?;
            let json: serde_json::Value = serde_json::from_reader(file).context(format!(
                "Failed to parse JSON from: {}",
                file_path.display()
            ))?;

            if let Some(version_value) = json.get("version") {
                let version_str = version_value
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Version in JSON is not a string"))?;
                Version::parse(version_str).context(format!(
                    "Failed to parse version '{}' as semver",
                    version_str
                ))
            } else {
                bail!("Cannot find 'version' field in {}", file_path.display());
            }
        }
        VersionFileFormat::Toml => {
            let toml: DocumentMut = fs::read_to_string(file_path)?
                .parse()
                .with_context(|| format!("Failed to read TOML file: {}", file_path.display()))?;

            // For Cargo.toml, version is under [package]
            if file_name == "Cargo.toml" {
                if let Some(package) = toml.get("package") {
                    if let Some(version_value) = package.get("version") {
                        let version_str = version_value
                            .as_str()
                            .ok_or_else(|| anyhow::anyhow!("Version in TOML is not a string"))?;
                        return Version::parse(version_str).context(format!(
                            "Failed to parse version '{}' as semver",
                            version_str
                        ));
                    }
                }
                bail!(
                    "Cannot find 'package.version' field in {}",
                    file_path.display()
                );
            } else {
                // For other TOML files, try to find version at the root
                if let Some(version_value) = toml.get("version") {
                    let version_str = version_value
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("Version in TOML is not a string"))?;
                    Version::parse(version_str).context(format!(
                        "Failed to parse version '{}' as semver",
                        version_str
                    ))
                } else {
                    bail!("Cannot find 'version' field in {}", file_path.display());
                }
            }
        }
    }
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

    let settings = init_settings(&project_repo.directory)?;

    let version_file_name = settings.version_file;

    let version = get_version_from_file(&project_repo.directory.join(&version_file_name))?;

    let prerelease_identifier = matches
        .get_one::<String>("pre_id")
        .map(|pre_id| format!("{pre_id}.0"))
        .unwrap_or("0".to_string());

    let mut next_version = if let Some(bump_type) = matches.get_one::<BumpType>("bump_type") {
        match bump_type {
            BumpType::Major => version.increment_major(),
            BumpType::Minor => version.increment_minor(),
            BumpType::Patch => version.increment_patch(),
            BumpType::PreMajor => version
                .increment_major()
                .append_prerelease_identifiers(&prerelease_identifier),
            BumpType::PreMinor => version
                .increment_minor()
                .append_prerelease_identifiers(&prerelease_identifier),
            BumpType::PrePatch => version
                .increment_patch()
                .append_prerelease_identifiers(&prerelease_identifier),
            BumpType::Prerelease => version.increment_prerelease(),
            BumpType::Release => version.convert_prerelease_to_release(),
        }
    } else {
        version.clone()
    };

    if version == next_version {
        debug!("no change in version, prompt");
        next_version = prompt_version_select(&version, &prerelease_identifier);
    }

    if version == next_version {
        debug!("just no change in version, exit");
        return Ok(());
    }

    let next_version = next_version.to_string();

    let mut skip_actions: Vec<Action> = matches
        .get_many::<Action>("skip")
        .unwrap_or_default()
        .copied()
        .collect();
    skip_actions.sort();
    skip_actions.dedup();

    if matches.get_flag("dryrun") {
        println!(
            "{} {}{}",
            "will bump version to".bg::<xterm::Gray>(),
            settings.tag_prefix.green(),
            next_version.green()
        );

        let file_names = std::iter::once(version_file_name.to_string())
            .chain(settings.bump_files)
            .collect::<Vec<_>>()
            .join(", ");

        println!(
            "{} {}",
            "will bump files".bg::<xterm::Gray>(),
            file_names.green(),
        );

        if !skip_actions.contains(&Action::Commit) {
            println!(
                "{} {}",
                "will commit files".bg::<xterm::Gray>(),
                file_names.green()
            );

            if !skip_actions.contains(&Action::Tag) {
                println!("{}", "will tag version".bg::<xterm::Gray>(),);
            }
        }

        return Ok(());
    }

    info!("bump to version {}", next_version);

    match detect_file_format(&project_repo.directory.join(&version_file_name))? {
        VersionFileFormat::Json => project_repo.bump_json(&version_file_name, &next_version)?,
        VersionFileFormat::Toml => project_repo.bump_toml(&version_file_name, &next_version)?,
    }

    project_repo.stage_file(&version_file_name)?;

    debug!("bump other files {:?}", settings.bump_files);

    for bump_file in settings.bump_files {
        if !Path::new(&bump_file).exists() {
            debug!("{bump_file} does not exist, skip.");
            continue;
        }

        match detect_file_format(&project_repo.directory.join(&bump_file))? {
            VersionFileFormat::Json => project_repo.bump_json(&bump_file, &next_version)?,
            VersionFileFormat::Toml => project_repo.bump_toml(&bump_file, &next_version)?,
        }

        project_repo.stage_file(&bump_file)?;
    }

    if !skip_actions.contains(&Action::Commit) {
        project_repo.commit_changes(&next_version)?;

        if !skip_actions.contains(&Action::Tag) {
            project_repo.tag_release(&next_version, &settings.tag_prefix)?;
        }
    }

    Ok(())
}
