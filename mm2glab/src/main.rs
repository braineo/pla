use clap::Parser;
use cli::Command;
use std::process;

pub mod api;
mod cli;
mod models;
mod services;
mod settings;

#[tokio::main]
async fn main() {
    let cli = cli::Cli::parse();

    let log_level = match &cli.command {
        Command::Issue(args) => args.common.log_level,
        Command::Fetch(args) => args.common.log_level,
    };

    env_logger::Builder::from_default_env()
        .filter_level(log_level.into())
        .format_timestamp_secs()
        .init();

    let result = match cli.command {
        Command::Issue(args) => services::create_issue(args).await,
        Command::Fetch(args) => services::fetch_thread(args).await,
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}
