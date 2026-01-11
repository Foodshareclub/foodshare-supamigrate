use crate::cli::{VaultArgs, VaultCommands};
use crate::config::Config;
use crate::db::{VaultBackup, VaultClient};
use anyhow::Result;
use console::style;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

pub fn run(args: VaultArgs) -> Result<()> {
    match args.command {
        VaultCommands::List { project } => list_secrets(&project),
        VaultCommands::Export { project, output } => export_secrets(&project, &output),
        VaultCommands::Import { project, file } => import_secrets(&project, &file),
        VaultCommands::Copy { from, to } => copy_secrets(&from, &to),
    }
}

fn list_secrets(project_name: &str) -> Result<()> {
    let config = Config::load(None)?;
    let project = config.get_project(project_name)?;

    let client = VaultClient::new(project.db_url());

    if !client.is_vault_enabled()? {
        println!(
            "{} Vault extension is not enabled in project '{}'",
            style("ℹ").blue(),
            project_name
        );
        println!("  Enable it with: CREATE EXTENSION IF NOT EXISTS supabase_vault");
        return Ok(());
    }

    let secrets = client.list_secrets()?;

    println!(
        "\n{} Vault Secrets in {} ({} found)",
        style("🔐").bold(),
        project_name,
        secrets.len()
    );
    println!("{:-<60}", "");

    if secrets.is_empty() {
        println!("  No vault secrets found");
    } else {
        for secret in secrets {
            let desc = secret.description.as_deref().unwrap_or("(no description)");
            println!(
                "  {} {} - {}",
                style("•").cyan(),
                style(&secret.name).bold(),
                style(desc).dim()
            );
        }
    }

    println!(
        "\n{} Vault secrets contain actual values (unlike edge function secrets)",
        style("ℹ").blue()
    );

    Ok(())
}

fn export_secrets(project_name: &str, output: &Path) -> Result<()> {
    let config = Config::load(None)?;
    let project = config.get_project(project_name)?;

    let client = VaultClient::new(project.db_url());

    if !client.is_vault_enabled()? {
        println!(
            "{} Vault extension is not enabled in project '{}'",
            style("⚠").yellow(),
            project_name
        );
        return Ok(());
    }

    let backup = client.backup()?;

    if backup.secrets.is_empty() {
        println!("{} No vault secrets to export", style("ℹ").blue());
        return Ok(());
    }

    // Security warning
    println!(
        "\n{} {} This file will contain DECRYPTED secret values!",
        style("⚠").yellow().bold(),
        style("WARNING:").yellow().bold()
    );
    println!("  Store it securely and delete after use.\n");

    print!("Proceed with export? [y/N] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if !input.trim().eq_ignore_ascii_case("y") {
        println!("{} Export cancelled", style("✗").red());
        return Ok(());
    }

    let json = serde_json::to_string_pretty(&backup)?;
    fs::write(output, json)?;

    println!(
        "\n{} Exported {} vault secrets to {}",
        style("✓").green(),
        backup.secrets.len(),
        output.display()
    );

    Ok(())
}

fn import_secrets(project_name: &str, file: &Path) -> Result<()> {
    let config = Config::load(None)?;
    let project = config.get_project(project_name)?;

    let client = VaultClient::new(project.db_url());

    if !client.is_vault_enabled()? {
        println!(
            "{} Vault extension is not enabled in project '{}'",
            style("⚠").yellow(),
            project_name
        );
        println!("  Enable it with: CREATE EXTENSION IF NOT EXISTS supabase_vault");
        return Ok(());
    }

    let content = fs::read_to_string(file)?;
    let backup: VaultBackup = serde_json::from_str(&content)?;

    if backup.secrets.is_empty() {
        println!("{} No secrets found in file", style("ℹ").blue());
        return Ok(());
    }

    println!(
        "\n{} Importing {} vault secrets to {}",
        style("🔐").bold(),
        backup.secrets.len(),
        project_name
    );

    for secret in &backup.secrets {
        let desc = secret.description.as_deref().unwrap_or("(no description)");
        println!("  {} {} - {}", style("•").cyan(), secret.name, desc);
    }

    print!("\nProceed? [y/N] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if !input.trim().eq_ignore_ascii_case("y") {
        println!("{} Import cancelled", style("✗").red());
        return Ok(());
    }

    let count = client.restore(&backup)?;

    println!(
        "\n{} Imported {} vault secrets (skipped {} existing)",
        style("✓").green(),
        count,
        backup.secrets.len() - count
    );

    Ok(())
}

fn copy_secrets(from_name: &str, to_name: &str) -> Result<()> {
    let config = Config::load(None)?;
    let source = config.get_project(from_name)?;
    let target = config.get_project(to_name)?;

    let source_client = VaultClient::new(source.db_url());
    let target_client = VaultClient::new(target.db_url());

    if !source_client.is_vault_enabled()? {
        println!(
            "{} Vault extension is not enabled in source project '{}'",
            style("⚠").yellow(),
            from_name
        );
        return Ok(());
    }

    if !target_client.is_vault_enabled()? {
        println!(
            "{} Vault extension is not enabled in target project '{}'",
            style("⚠").yellow(),
            to_name
        );
        println!("  Enable it with: CREATE EXTENSION IF NOT EXISTS supabase_vault");
        return Ok(());
    }

    let backup = source_client.backup()?;

    if backup.secrets.is_empty() {
        println!(
            "{} No vault secrets found in {}",
            style("ℹ").blue(),
            from_name
        );
        return Ok(());
    }

    println!(
        "\n{} Copying {} vault secrets from {} to {}",
        style("🔐").bold(),
        backup.secrets.len(),
        from_name,
        to_name
    );

    for secret in &backup.secrets {
        let desc = secret.description.as_deref().unwrap_or("(no description)");
        println!("  {} {} - {}", style("•").cyan(), secret.name, desc);
    }

    print!("\nProceed? [y/N] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if !input.trim().eq_ignore_ascii_case("y") {
        println!("{} Copy cancelled", style("✗").red());
        return Ok(());
    }

    let count = target_client.restore(&backup)?;

    println!(
        "\n{} Copied {} vault secrets (skipped {} existing)",
        style("✓").green(),
        count,
        backup.secrets.len() - count
    );

    Ok(())
}

/// Backup vault secrets from a project (called by backup command)
pub fn backup_vault(project_name: &str) -> Result<Option<VaultBackup>> {
    let config = Config::load(None)?;
    let project = config.get_project(project_name)?;

    let client = VaultClient::new(project.db_url());

    if !client.is_vault_enabled()? {
        return Ok(None);
    }

    let backup = client.backup()?;
    if backup.secrets.is_empty() {
        return Ok(None);
    }

    Ok(Some(backup))
}

/// Restore vault secrets from backup
pub fn restore_vault(backup: &VaultBackup, project_name: &str) -> Result<usize> {
    let config = Config::load(None)?;
    let project = config.get_project(project_name)?;

    let client = VaultClient::new(project.db_url());

    if !client.is_vault_enabled()? {
        return Err(anyhow::anyhow!(
            "Vault extension is not enabled in target project. Enable it with: CREATE EXTENSION IF NOT EXISTS supabase_vault"
        ));
    }

    Ok(client.restore(backup)?)
}
