use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod cli;
mod commands;
mod config;
mod db;
mod error;
mod functions;
mod storage;

use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive("supamigrate=info".parse()?))
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Migrate(args) => commands::migrate::run(args).await,
        Commands::Backup(args) => commands::backup::run(args).await,
        Commands::Restore(args) => commands::restore::run(args).await,
        Commands::Storage(args) => commands::storage::run(args).await,
        Commands::Config(args) => commands::config::run(args),
    }
}
