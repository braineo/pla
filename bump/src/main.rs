use crate::{repo::Repo, settings::Settings};
use anyhow::bail;
use bump_version::{BumpType, BumpVersion};
use clap::{value_parser, Arg, ArgAction, Command, ValueEnum};
use clap_complete::{generate, Generator, Shell};
use cli::prompt_version_select;
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
        .add_source(config::File::from(project_repo.directory.join("bump")).required(false))
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

    let mut next_version = if let Some(bump_type) = matches.get_one::<BumpType>("bump_type") {
        match bump_type {
            BumpType::Major => version.increment_major(),
            BumpType::Minor => version.increment_minor(),
            BumpType::Patch => version.increment_patch(),
        }
    } else {
        version.clone()
    };

    next_version = if let Some(prerelease) = matches.get_one::<String>("prerelease") {
        if next_version.pre.is_empty() {
            if prerelease.is_empty() {
                next_version.append_prerelease_identifiers("0")
            } else {
                next_version.append_prerelease_identifiers(&format!("{prerelease}.0"))
            }
        } else if !prerelease.is_empty() {
            bail!(
                "prerelease identifiers exists {} but got specified as {}",
                next_version.pre,
                prerelease
            );
        } else {
            next_version.increment_prerelease()
        }
    } else {
        next_version.convert_prerelease_to_release()
    };

    if version == next_version {
        debug!("no change in version, prompt");
        next_version = prompt_version_select(&version);
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

        let file_names = std::iter::once(package_json_file_name.to_string())
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
    project_repo.bump_json(package_json_file_name, &next_version)?;
    project_repo.stage_file(package_json_file_name)?;

    debug!("bump other files {:?}", settings.bump_files);

    for bump_file in settings.bump_files {
        if !Path::new(&bump_file).exists() {
            debug!("{bump_file} does not exist, skip.");
            continue;
        }

        project_repo.bump_json(&bump_file, &next_version)?;
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
