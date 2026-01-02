use crate::cli::MigrateArgs;
use crate::config::Config;
use crate::db::{PgDump, PgRestore, SqlTransformer};
use crate::storage::{StorageClient, StorageTransfer};
use anyhow::Result;
use console::style;
use std::io::{self, Write};
use tempfile::NamedTempFile;
use tracing::info;

pub async fn run(args: MigrateArgs) -> Result<()> {
    let config = Config::load(None)?;

    let source = config.get_project(&args.from)?;
    let target = config.get_project(&args.to)?;

    println!(
        "\n{} Migration Plan",
        style("📋").bold()
    );
    println!("  Source: {} ({})", args.from, source.project_ref);
    println!("  Target: {} ({})", args.to, target.project_ref);
    println!("  Schema only: {}", args.schema_only);
    println!("  Data only: {}", args.data_only);
    println!("  Include storage: {}", args.include_storage);

    if args.dry_run {
        println!("\n{} Dry run - no changes will be made", style("ℹ️").cyan());
        return Ok(());
    }

    if !args.yes {
        print!("\nProceed with migration? [y/N] ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Migration cancelled.");
            return Ok(());
        }
    }

    // Database migration
    println!("\n{} Starting database migration...", style("🗄️").bold());

    let excluded_schemas = args
        .exclude_schemas
        .unwrap_or_else(|| config.defaults.excluded_schemas.clone());

    let excluded_tables = args.exclude_tables.unwrap_or_default();

    // Dump source database
    info!("Dumping source database...");
    let dump = PgDump::new(source.db_url())
        .exclude_schemas(excluded_schemas)
        .exclude_tables(excluded_tables)
        .schema_only(args.schema_only)
        .data_only(args.data_only)
        .dump_to_string()?;

    // Transform SQL for Supabase compatibility
    info!("Transforming SQL...");
    let transformed = SqlTransformer::transform(&dump);

    // Write to temp file
    let temp_file = NamedTempFile::new()?;
    std::fs::write(temp_file.path(), &transformed)?;

    // Restore to target
    info!("Restoring to target database...");
    let restore = PgRestore::new(target.db_url());
    restore.restore_from_file(temp_file.path())?;

    println!("{} Database migration complete!", style("✓").green());

    // Storage migration
    if args.include_storage {
        println!("\n{} Starting storage migration...", style("📦").bold());

        let source_key = source.service_key.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Source project requires service_key for storage migration")
        })?;
        let target_key = target.service_key.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Target project requires service_key for storage migration")
        })?;

        let source_storage = StorageClient::new(source.api_url(), source_key.clone());
        let target_storage = StorageClient::new(target.api_url(), target_key.clone());

        let transfer = StorageTransfer::new(source_storage)
            .with_target(target_storage)
            .parallel(config.defaults.parallel_transfers);

        let stats = transfer.sync_all().await?;
        println!("{} Storage migration complete: {}", style("✓").green(), stats);
    }

    println!(
        "\n{} Migration completed successfully!",
        style("🎉").bold()
    );

    Ok(())
}
