use clap::{value_parser, Arg, Command};
use comfy_table::Table;
use log::{debug, info, warn, LevelFilter};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs,
    path::PathBuf,
};

#[derive(Debug, Serialize, Deserialize, Default, Clone, Eq, PartialEq)]
pub struct PackageLockJson {
    pub name: String,
    pub version: Option<String>,
    #[serde(rename = "lockfileVersion")]
    pub lockfile_version: u32,
    pub packages: Option<HashMap<String, Dependency>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Default)]
pub struct Dependency {
    pub version: String,
    pub name: Option<String>,
    pub resolved: Option<String>,
    pub integrity: Option<String>,
    #[serde(default)]
    pub bundled: bool,
    #[serde(rename = "dev", default)]
    pub is_dev: bool,
    #[serde(rename = "optional", default)]
    pub is_optional: bool,
    #[serde(rename = "devOptional", default)]
    pub is_dev_optional: bool,
    #[serde(rename = "inBundle", default)]
    pub is_in_bundle: bool,
    #[serde(rename = "hasInstallScript", default)]
    pub has_install_script: bool,
    #[serde(rename = "hasShrinkwrap", default)]
    pub has_shrink_wrap: bool,
    pub dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "devDependencies")]
    pub dev_dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "optionalDependencies")]
    pub optional_dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "peerDependencies")]
    pub peer_dependencies: Option<HashMap<String, String>>,
    pub license: Option<String>,
    // engines can be map or vec
    // pub engines: Option<HashMap<String, String>>,
    pub bin: Option<HashMap<String, String>>,
}

fn cli() -> Command {
    Command::new("package-lock-analyzer")
        .bin_name("pla")
        .about("analyze package lock for duplicated packages")
        .arg(
            Arg::new("path")
                .help("path to package-lock.json")
                .value_name("FILE")
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("log-level")
                .help("log level, can be trace, debug, info, warn, error")
                .short('l')
                .long("log-level")
                .value_name("LEVEL")
                .default_value("info"),
        )
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = cli().get_matches();
    let mut log_level = log::LevelFilter::Debug;

    if let Some(user_log_level) = matches.get_one::<String>("log-level") {
        log_level = match user_log_level.as_str() {
            "trace" => LevelFilter::Trace,
            "debug" => LevelFilter::Debug,
            "info" => LevelFilter::Info,
            "warn" => LevelFilter::Warn,
            "error" => LevelFilter::Error,
            _ => LevelFilter::Info,
        }
    }

    env_logger::builder().filter_level(log_level).init();

    if let Some(package_lock_path) = matches.get_one::<PathBuf>("path") {
        info!("reading package lock from {}", package_lock_path.display());
        let file = fs::File::open(package_lock_path)?;
        let lock_file: PackageLockJson = serde_json::from_reader(file)?;

        let mut package_versions: HashMap<String, HashSet<String>> = HashMap::new();
        match lock_file.packages {
            Some(packages) => {
                for (package_install_path, dependency) in packages {
                    debug!(
                        "name: {}, version: {}",
                        package_install_path, dependency.version
                    );

                    let package_name = package_install_path
                        .rsplit("node_modules/")
                        .next()
                        .unwrap_or("unknown");

                    let versions = package_versions
                        .entry(package_name.to_string())
                        .or_default();
                    versions.insert(dependency.version);
                }
            }
            None => {
                warn!("no packages to iterate")
            }
        }

        let diverged_count: usize = package_versions
            .values()
            .map(|value| if value.len() > 1 { 1 } else { 0 })
            .sum();

        info!(
            "total {} of distinct package installed. {} packages have different versions",
            package_versions.len(),
            diverged_count
        );

        let mut table = Table::new();

        table.set_header(vec!["package", "versions"]);

        let mut filtered_rows: Vec<_> = package_versions
            .iter()
            .filter_map(|(package_name, versions)| {
                if versions.len() > 1 {
                    let mut version_vec = Vec::from_iter(versions);
                    version_vec.sort();

                    Some((
                        package_name.clone(),
                        version_vec
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                            .join(", "),
                    ))
                } else {
                    None
                }
            })
            .collect();

        filtered_rows.sort_by_key(|(name, _)| name.clone());

        for (package_name, versions) in filtered_rows {
            if versions.len() > 1 {
                table.add_row(vec![package_name, versions]);
            }
        }
        println!("{table}")
    }
    Ok(())
}
