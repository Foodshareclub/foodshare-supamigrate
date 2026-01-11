use crate::cli::RestoreArgs;
use crate::commands::secrets::restore_secrets;
use crate::commands::vault::restore_vault;
use crate::config::Config;
use crate::db::{PgRestore, SqlTransformer, VaultBackup};
use crate::error::SupamigrateError;
use crate::functions::secrets::SecretsBackup;
use crate::functions::{FunctionBackup, FunctionFile, FunctionsClient};
use crate::storage::StorageClient;
use anyhow::Result;
use console::style;
use std::fs;
use std::io::{self, Read, Write};
use tracing::info;

#[derive(serde::Deserialize)]
struct BackupMetadata {
    #[allow(dead_code)]
    project_ref: String,
    #[allow(dead_code)]
    timestamp: String,
    #[allow(dead_code)]
    schema_only: bool,
    include_storage: bool,
    #[serde(default)]
    include_functions: bool,
    #[serde(default)]
    include_secrets: bool,
    #[serde(default)]
    secrets_count: usize,
    #[serde(default)]
    include_vault: bool,
    #[serde(default)]
    vault_count: usize,
    compressed: bool,
}

pub async fn run(args: RestoreArgs) -> Result<()> {
    let config = Config::load(None)?;
    let target = config.get_project(&args.to)?;

    // Validate backup exists
    if !args.from.exists() {
        return Err(SupamigrateError::BackupNotFound(args.from.display().to_string()).into());
    }

    // Load metadata
    let metadata_path = args.from.join("metadata.json");
    let metadata: BackupMetadata = if metadata_path.exists() {
        let content = fs::read_to_string(&metadata_path)?;
        serde_json::from_str(&content)?
    } else {
        return Err(SupamigrateError::InvalidBackup("metadata.json not found".to_string()).into());
    };

    println!("\n{} Restore Plan", style("📋").bold());
    println!("  From: {}", args.from.display());
    println!("  Target: {} ({})", args.to, target.project_ref);
    println!(
        "  Include storage: {}",
        args.include_storage && metadata.include_storage
    );
    println!(
        "  Include functions: {}",
        args.include_functions && metadata.include_functions
    );
    println!(
        "  Include secrets: {} ({})",
        args.include_secrets && metadata.include_secrets,
        if metadata.include_secrets {
            format!("{} secret names in backup", metadata.secrets_count)
        } else {
            "no secrets in backup".to_string()
        }
    );
    if args.include_secrets && metadata.include_secrets {
        if let Some(ref secrets_file) = args.secrets_file {
            println!("  Secrets file: {}", secrets_file.display());
        } else {
            println!("  Secrets file: (will prompt for values)");
        }
    }
    println!(
        "  Include vault: {} ({})",
        args.include_vault && metadata.include_vault,
        if metadata.include_vault {
            format!("{} vault secrets in backup", metadata.vault_count)
        } else {
            "no vault secrets in backup".to_string()
        }
    );

    if !args.yes {
        print!("\n⚠️  This will overwrite data in the target project. Proceed? [y/N] ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Restore cancelled.");
            return Ok(());
        }
    }

    // Database restore
    println!("\n{} Restoring database...", style("🗄️").bold());

    let dump_file = if metadata.compressed {
        args.from.join("database.sql.gz")
    } else {
        args.from.join("database.sql")
    };

    if !dump_file.exists() {
        return Err(SupamigrateError::InvalidBackup(format!(
            "Database dump not found: {}",
            dump_file.display()
        ))
        .into());
    }

    let sql = if metadata.compressed {
        let file = fs::File::open(&dump_file)?;
        let mut decoder = flate2::read::GzDecoder::new(file);
        let mut content = String::new();
        decoder.read_to_string(&mut content)?;
        content
    } else {
        fs::read_to_string(&dump_file)?
    };

    // Transform SQL for Supabase compatibility
    info!("Transforming SQL...");
    let transformed = SqlTransformer::transform(&sql);

    // Restore to target
    info!("Restoring to target database...");
    let restore = PgRestore::new(target.db_url());
    restore.restore_from_string(&transformed)?;

    println!("{} Database restore complete!", style("✓").green());

    // Storage restore
    if args.include_storage && metadata.include_storage {
        println!("\n{} Restoring storage...", style("📦").bold());

        let service_key = target.service_key.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Target project requires service_key for storage restore")
        })?;

        let storage = StorageClient::new(target.api_url(), service_key.clone());
        let storage_dir = args.from.join("storage");

        if storage_dir.exists() {
            let stats = restore_storage(&storage, &storage_dir).await?;
            println!("{} Storage restore complete: {}", style("✓").green(), stats);
        } else {
            println!("{} No storage backup found, skipping", style("⚠️").yellow());
        }
    }

    // Edge Functions restore
    if args.include_functions && metadata.include_functions {
        println!("\n{} Restoring edge functions...", style("⚡").bold());

        let service_key = target.service_key.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Target project requires service_key for edge functions restore")
        })?;

        let functions_client =
            FunctionsClient::new(target.project_ref.clone(), service_key.clone());

        let functions_dir = args.from.join("functions");

        if functions_dir.exists() {
            let stats = restore_functions(&functions_client, &functions_dir).await?;
            println!(
                "{} Edge functions restore complete: {}",
                style("✓").green(),
                stats
            );
        } else {
            println!(
                "{} No functions backup found, skipping",
                style("⚠️").yellow()
            );
        }
    }

    // Secrets restore
    if args.include_secrets && metadata.include_secrets {
        println!("\n{} Restoring secrets...", style("🔐").bold());

        let secrets_file = args.from.join("secrets.json");

        if secrets_file.exists() {
            let secrets_content = fs::read_to_string(&secrets_file)?;
            let secrets_backup: SecretsBackup = serde_json::from_str(&secrets_content)?;

            if secrets_backup.secrets.is_empty() {
                println!("{} No secrets in backup, skipping", style("ℹ").blue());
            } else {
                let count =
                    restore_secrets(&secrets_backup, &args.to, args.secrets_file.as_deref())
                        .await?;

                if count > 0 {
                    println!(
                        "{} Secrets restore complete: {} secrets set",
                        style("✓").green(),
                        count
                    );
                } else {
                    println!(
                        "{} No secrets were set (all skipped or empty values)",
                        style("⚠").yellow()
                    );
                }
            }
        } else {
            println!("{} No secrets backup found, skipping", style("⚠️").yellow());
        }
    }

    // Vault restore
    if args.include_vault && metadata.include_vault {
        println!("\n{} Restoring vault secrets...", style("🔐").bold());

        let vault_file = args.from.join("vault_secrets.json");

        if vault_file.exists() {
            let vault_content = fs::read_to_string(&vault_file)?;
            let vault_backup: VaultBackup = serde_json::from_str(&vault_content)?;

            if vault_backup.secrets.is_empty() {
                println!("{} No vault secrets in backup, skipping", style("ℹ").blue());
            } else {
                match restore_vault(&vault_backup, &args.to) {
                    Ok(count) => {
                        println!(
                            "{} Vault restore complete: {} secrets created (skipped {} existing)",
                            style("✓").green(),
                            count,
                            vault_backup.secrets.len() - count
                        );
                    }
                    Err(e) => {
                        println!("{} Vault restore failed: {}", style("⚠").yellow(), e);
                    }
                }
            }
        } else {
            println!("{} No vault backup found, skipping", style("⚠️").yellow());
        }
    }

    println!("\n{} Restore completed successfully!", style("🎉").bold());

    Ok(())
}

async fn restore_functions(
    client: &FunctionsClient,
    functions_dir: &std::path::Path,
) -> Result<FunctionsRestoreStats> {
    let mut stats = FunctionsRestoreStats::default();

    let entries = fs::read_dir(functions_dir)?;
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let func_dir = entry.path();
            let metadata_path = func_dir.join("metadata.json");

            if !metadata_path.exists() {
                continue;
            }

            // Read function metadata
            let metadata_content = fs::read_to_string(&metadata_path)?;
            let metadata: serde_json::Value = serde_json::from_str(&metadata_content)?;

            let slug = metadata["slug"].as_str().unwrap_or_default().to_string();
            let name = metadata["name"].as_str().unwrap_or(&slug).to_string();
            let verify_jwt = metadata["verify_jwt"].as_bool().unwrap_or(true);
            let entrypoint_path = metadata["entrypoint_path"].as_str().map(String::from);
            let import_map_path = metadata["import_map_path"].as_str().map(String::from);

            // Read function files
            let mut files = Vec::new();
            read_function_files(&func_dir, &func_dir, &mut files)?;

            // Filter out metadata.json
            let files: Vec<FunctionFile> = files
                .into_iter()
                .filter(|f| f.name != "metadata.json")
                .collect();

            if files.is_empty() {
                continue;
            }

            let backup = FunctionBackup {
                slug: slug.clone(),
                name,
                verify_jwt,
                entrypoint_path,
                import_map_path,
                files,
            };

            info!("Deploying function: {}", slug);
            client.deploy_function(&backup).await?;
            stats.functions += 1;
        }
    }

    Ok(stats)
}

fn read_function_files(
    base_dir: &std::path::Path,
    current_dir: &std::path::Path,
    files: &mut Vec<FunctionFile>,
) -> Result<()> {
    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            read_function_files(base_dir, &path, files)?;
        } else if path.is_file() {
            let relative_path = path
                .strip_prefix(base_dir)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            let content = fs::read_to_string(&path)?;
            files.push(FunctionFile {
                name: relative_path,
                content,
            });
        }
    }
    Ok(())
}

async fn restore_storage(
    client: &StorageClient,
    storage_dir: &std::path::Path,
) -> Result<RestoreStats> {
    use tokio::fs;

    let mut stats = RestoreStats::default();

    let mut entries = fs::read_dir(storage_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_dir() {
            let bucket_name = entry.file_name().to_string_lossy().to_string();

            // Create bucket (assume public for now, could store in metadata)
            client.create_bucket(&bucket_name, false).await?;
            stats.buckets += 1;

            // Upload files
            let bucket_dir = entry.path();
            let mut files = fs::read_dir(&bucket_dir).await?;

            while let Some(file_entry) = files.next_entry().await? {
                if file_entry.file_type().await?.is_file() {
                    let file_name = file_entry.file_name().to_string_lossy().to_string();
                    let data = fs::read(file_entry.path()).await?;
                    let data_len = data.len();

                    client.upload(&bucket_name, &file_name, data.into()).await?;
                    stats.objects += 1;
                    stats.bytes += data_len;
                }
            }
        }
    }

    Ok(stats)
}

#[derive(Default)]
struct RestoreStats {
    buckets: usize,
    objects: usize,
    bytes: usize,
}

impl std::fmt::Display for RestoreStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} buckets, {} objects restored",
            self.buckets, self.objects
        )
    }
}

#[derive(Default)]
struct FunctionsRestoreStats {
    functions: usize,
}

impl std::fmt::Display for FunctionsRestoreStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} functions deployed", self.functions)
    }
}
