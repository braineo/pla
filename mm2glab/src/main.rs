use clap::Parser;
use std::process;

mod api;
mod cli;
mod models;
mod services;

#[tokio::main]
async fn main() {
    let args = cli::Args::parse();

    if let Err(e) = services::run(args).await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
