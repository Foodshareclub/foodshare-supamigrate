use crate::cli::BackupArgs;
use crate::commands::secrets::backup_secrets;
use crate::commands::vault::backup_vault;
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
    println!("  Include vault: {}", args.include_vault);
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

    // Secrets backup (if access_token available)
    let mut secrets_count = 0;
    if project.has_secrets_access() {
        println!("\n{} Backing up secrets...", style("🔐").bold());

        match backup_secrets(&args.project).await? {
            Some(secrets_backup) => {
                secrets_count = secrets_backup.secrets.len();
                let secrets_file = backup_dir.join("secrets.json");
                fs::write(
                    &secrets_file,
                    serde_json::to_string_pretty(&secrets_backup)?,
                )?;
                info!("Secrets backup saved to: {}", secrets_file.display());
                println!(
                    "{} Secrets backup complete: {} secret names (values not backed up for security)",
                    style("✓").green(),
                    secrets_count
                );
            }
            None => {
                println!(
                    "{} Skipping secrets (no access_token configured)",
                    style("⚠").yellow()
                );
            }
        }
    } else {
        println!(
            "\n{} Skipping secrets backup (no access_token configured)",
            style("ℹ").blue()
        );
        println!(
            "  Add access_token to config to backup secret names: https://supabase.com/dashboard/account/tokens"
        );
    }

    // Vault backup (if --include-vault flag is set)
    let mut vault_count = 0;
    if args.include_vault {
        println!("\n{} Backing up vault secrets...", style("🔐").bold());

        match backup_vault(&args.project) {
            Ok(Some(vault_backup)) => {
                vault_count = vault_backup.secrets.len();
                let vault_file = backup_dir.join("vault_secrets.json");
                fs::write(&vault_file, serde_json::to_string_pretty(&vault_backup)?)?;
                info!("Vault backup saved to: {}", vault_file.display());
                println!(
                    "{} Vault backup complete: {} secrets (with values)",
                    style("✓").green(),
                    vault_count
                );
                println!(
                    "  {} vault_secrets.json contains decrypted values - store securely!",
                    style("⚠").yellow()
                );
            }
            Ok(None) => {
                println!(
                    "{} No vault secrets found or vault not enabled",
                    style("ℹ").blue()
                );
            }
            Err(e) => {
                println!("{} Vault backup failed: {}", style("⚠").yellow(), e);
            }
        }
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
        include_secrets: secrets_count > 0,
        secrets_count,
        include_vault: vault_count > 0,
        vault_count,
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
    include_secrets: bool,
    secrets_count: usize,
    include_vault: bool,
    vault_count: usize,
    compressed: bool,
}
