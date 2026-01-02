use crate::cli::BackupArgs;
use crate::config::Config;
use crate::db::PgDump;
use crate::functions::FunctionsClient;
use crate::storage::{StorageClient, StorageTransfer};
use anyhow::Result;
use chrono::Utc;
use console::style;
use std::fs;
use std::io::Write;
use tracing::info;

pub async fn run(args: BackupArgs) -> Result<()> {
    let config = Config::load(None)?;
    let project = config.get_project(&args.project)?;

    // Create output directory with timestamp
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let backup_dir = args.output.join(format!("{}_{}", args.project, timestamp));
    fs::create_dir_all(&backup_dir)?;

    let include_functions = !args.no_functions;

    println!("\n{} Backup Plan", style("📋").bold());
    println!("  Project: {} ({})", args.project, project.project_ref);
    println!("  Output: {}", backup_dir.display());
    println!("  Schema only: {}", args.schema_only);
    println!("  Include storage: {}", args.include_storage);
    println!("  Include functions: {}", include_functions);
    println!("  Compress: {}", args.compress);

    // Database backup
    println!("\n{} Backing up database...", style("🗄️").bold());

    let dump_file = if args.compress {
        backup_dir.join("database.sql.gz")
    } else {
        backup_dir.join("database.sql")
    };

    let dump = PgDump::new(project.db_url())
        .exclude_schemas(config.defaults.excluded_schemas.clone())
        .schema_only(args.schema_only)
        .dump_to_string()?;

    if args.compress {
        use std::io::BufWriter;
        let file = fs::File::create(&dump_file)?;
        let mut encoder =
            flate2::write::GzEncoder::new(BufWriter::new(file), flate2::Compression::default());
        encoder.write_all(dump.as_bytes())?;
        encoder.finish()?;
    } else {
        fs::write(&dump_file, &dump)?;
    }

    info!("Database backup saved to: {}", dump_file.display());
    println!("{} Database backup complete!", style("✓").green());

    // Edge Functions backup (included by default)
    if include_functions {
        println!("\n{} Backing up edge functions...", style("⚡").bold());

        let service_key = project.service_key.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Project requires service_key for edge functions backup")
        })?;

        let functions_client =
            FunctionsClient::new(project.project_ref.clone(), service_key.clone());

        let functions = functions_client.backup_all().await?;
        let functions_dir = backup_dir.join("functions");
        fs::create_dir_all(&functions_dir)?;

        for func in &functions {
            let func_dir = functions_dir.join(&func.slug);
            fs::create_dir_all(&func_dir)?;

            // Save function metadata
            let metadata = serde_json::json!({
                "slug": func.slug,
                "name": func.name,
                "verify_jwt": func.verify_jwt,
                "entrypoint_path": func.entrypoint_path,
                "import_map_path": func.import_map_path,
            });
            fs::write(
                func_dir.join("metadata.json"),
                serde_json::to_string_pretty(&metadata)?,
            )?;

            // Save function files
            for file in &func.files {
                let file_path = func_dir.join(&file.name);
                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&file_path, &file.content)?;
            }

            info!("Backed up function: {}", func.slug);
        }

        println!(
            "{} Edge functions backup complete: {} functions",
            style("✓").green(),
            functions.len()
        );
    }

    // Storage backup
    if args.include_storage {
        println!("\n{} Backing up storage...", style("📦").bold());

        let service_key = project
            .service_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Project requires service_key for storage backup"))?;

        let storage = StorageClient::new(project.api_url(), service_key.clone());
        let storage_dir = backup_dir.join("storage");
        fs::create_dir_all(&storage_dir)?;

        let transfer = StorageTransfer::new(storage).parallel(config.defaults.parallel_transfers);

        let stats = transfer.download_all(&storage_dir).await?;
        println!("{} Storage backup complete: {}", style("✓").green(), stats);
    }

    // Write metadata
    let metadata = BackupMetadata {
        project_ref: project.project_ref.clone(),
        timestamp: Utc::now().to_rfc3339(),
        schema_only: args.schema_only,
        include_storage: args.include_storage,
        include_functions,
        compressed: args.compress,
    };

    let metadata_file = backup_dir.join("metadata.json");
    fs::write(&metadata_file, serde_json::to_string_pretty(&metadata)?)?;

    println!("\n{} Backup completed successfully!", style("🎉").bold());
    println!("  Location: {}", backup_dir.display());

    Ok(())
}

#[derive(serde::Serialize)]
struct BackupMetadata {
    project_ref: String,
    timestamp: String,
    schema_only: bool,
    include_storage: bool,
    include_functions: bool,
    compressed: bool,
}
