// Allow common pedantic lints that are too noisy for this codebase
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::single_match_else)]

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
        Commands::Secrets(args) => commands::secrets::run(args).await,
        Commands::Vault(args) => commands::vault::run(args),
        Commands::Config(args) => commands::config::run(args),
        Commands::Doctor(args) => {
            commands::doctor::run(commands::doctor::DoctorArgs { fix: args.fix })
        }
    }
}
