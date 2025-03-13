use clap::Parser;
use std::process;

mod api;
mod cli;
mod models;
mod services;
mod settings;

#[tokio::main]
async fn main() {
    let args = cli::Args::parse();

    env_logger::Builder::from_default_env()
        .filter_level(args.log_level.into())
        .format_timestamp_secs()
        .init();

    if let Err(e) = services::run(args).await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
