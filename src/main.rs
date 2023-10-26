use clap::{value_parser, Arg, Command};
use log::info;
use std::path::PathBuf;

fn cli() -> Command {
    Command::new("package-lock-analyzer")
        .bin_name("pla")
        .about("analyze package lock for duplicated packages")
        .arg(
            Arg::new("path")
                .value_name("FILE")
                .value_parser(value_parser!(PathBuf)),
        )
}

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let matches = cli().get_matches();

    if let Some(package_lock_path) = matches.get_one::<PathBuf>("path") {
        info!("reading package lock from {}", package_lock_path.display());
    }
}
