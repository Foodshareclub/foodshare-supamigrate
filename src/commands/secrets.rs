use crate::cli::{SecretsArgs, SecretsCommands};
use crate::config::Config;
use crate::functions::secrets::{
    generate_env_template, parse_env_file, Secret, SecretsBackup, SecretsClient,
};
use anyhow::Result;
use console::style;
use std::io::{self, Write};
use std::path::Path;

pub async fn run(args: SecretsArgs) -> Result<()> {
    match args.command {
        SecretsCommands::List { project } => list_secrets(&project).await,
        SecretsCommands::Export { project, output } => export_secrets(&project, &output).await,
        SecretsCommands::Import { project, file } => import_secrets(&project, &file).await,
        SecretsCommands::Copy { from, to } => copy_secrets(&from, &to).await,
    }
}

async fn list_secrets(project_name: &str) -> Result<()> {
    let config = Config::load(None)?;
    let project = config.get_project(project_name)?;

    let access_token = project
        .access_token
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Project requires access_token for secrets operations. Get one at: https://supabase.com/dashboard/account/tokens"))?;

    let client = SecretsClient::new(project.project_ref.clone(), access_token.clone());
    let secrets = client.list_secrets().await?;

    println!(
        "\n{} Secrets in {} ({} found)",
        style("🔐").bold(),
        project_name,
        secrets.len()
    );
    println!("{:-<50}", "");

    if secrets.is_empty() {
        println!("  No secrets found");
    } else {
        for secret in secrets {
            println!("  {} {}", style("•").cyan(), secret.name);
        }
    }

    println!(
        "\n{} Note: Secret values are never exposed via API",
        style("ℹ").blue()
    );

    Ok(())
}

async fn export_secrets(project_name: &str, output: &Path) -> Result<()> {
    let config = Config::load(None)?;
    let project = config.get_project(project_name)?;

    let access_token = project
        .access_token
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Project requires access_token for secrets operations"))?;

    let client = SecretsClient::new(project.project_ref.clone(), access_token.clone());
    let secrets = client.list_secrets().await?;

    let template = generate_env_template(&secrets);
    std::fs::write(output, template)?;

    println!(
        "{} Exported {} secret names to {}",
        style("✓").green(),
        secrets.len(),
        output.display()
    );
    println!(
        "{} Fill in the values and use: supamigrate secrets import --project <name> --file {}",
        style("ℹ").blue(),
        output.display()
    );

    Ok(())
}

async fn import_secrets(project_name: &str, file: &Path) -> Result<()> {
    let config = Config::load(None)?;
    let project = config.get_project(project_name)?;

    let access_token = project
        .access_token
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Project requires access_token for secrets operations"))?;

    let content = std::fs::read_to_string(file)?;
    let secrets = parse_env_file(&content);

    if secrets.is_empty() {
        println!("{} No secrets found in file", style("⚠").yellow());
        return Ok(());
    }

    // Filter out empty values
    let secrets_with_values: Vec<_> = secrets.iter().filter(|s| !s.value.is_empty()).collect();
    let empty_count = secrets.len() - secrets_with_values.len();

    if secrets_with_values.is_empty() {
        println!(
            "{} All secrets have empty values. Please fill in the values in {}",
            style("⚠").yellow(),
            file.display()
        );
        return Ok(());
    }

    println!(
        "\n{} Importing {} secrets to {} (skipping {} with empty values)",
        style("🔐").bold(),
        secrets_with_values.len(),
        project_name,
        empty_count
    );

    for secret in &secrets_with_values {
        println!("  {} {}", style("•").cyan(), secret.name);
    }

    print!("\nProceed? [y/N] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if !input.trim().eq_ignore_ascii_case("y") {
        println!("{} Cancelled", style("✗").red());
        return Ok(());
    }

    let client = SecretsClient::new(project.project_ref.clone(), access_token.clone());

    let secrets_to_create: Vec<Secret> = secrets_with_values
        .into_iter()
        .map(|s| Secret {
            name: s.name.clone(),
            value: s.value.clone(),
        })
        .collect();

    client.create_secrets(&secrets_to_create).await?;

    println!(
        "\n{} Successfully imported {} secrets",
        style("✓").green(),
        secrets_to_create.len()
    );

    Ok(())
}

async fn copy_secrets(from_name: &str, to_name: &str) -> Result<()> {
    let config = Config::load(None)?;
    let source = config.get_project(from_name)?;
    let target = config.get_project(to_name)?;

    let source_token = source
        .access_token
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Source project requires access_token"))?;
    let target_token = target
        .access_token
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Target project requires access_token"))?;

    let source_client = SecretsClient::new(source.project_ref.clone(), source_token.clone());
    let secrets = source_client.list_secrets().await?;

    if secrets.is_empty() {
        println!("{} No secrets found in {}", style("ℹ").blue(), from_name);
        return Ok(());
    }

    println!(
        "\n{} Copying {} secrets from {} to {}",
        style("🔐").bold(),
        secrets.len(),
        from_name,
        to_name
    );
    println!(
        "{} You will need to enter the value for each secret",
        style("ℹ").blue()
    );
    println!("{:-<50}", "");

    let mut secrets_to_create = Vec::new();

    for secret in &secrets {
        print!("  {} [press Enter to skip]: ", style(&secret.name).cyan());
        io::stdout().flush()?;

        let value = read_password()?;

        if value.is_empty() {
            println!("    {}", style("(skipped)").dim());
        } else {
            secrets_to_create.push(Secret {
                name: secret.name.clone(),
                value,
            });
            println!("    {}", style("(set)").green());
        }
    }

    if secrets_to_create.is_empty() {
        println!("\n{} No secrets to copy (all skipped)", style("ℹ").blue());
        return Ok(());
    }

    let target_client = SecretsClient::new(target.project_ref.clone(), target_token.clone());
    target_client.create_secrets(&secrets_to_create).await?;

    println!(
        "\n{} Copied {} secrets to {}",
        style("✓").green(),
        secrets_to_create.len(),
        to_name
    );

    Ok(())
}

/// Read password/secret from stdin
fn read_password() -> Result<String> {
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

/// Backup secrets from a project (called by backup command)
pub async fn backup_secrets(project_name: &str) -> Result<Option<SecretsBackup>> {
    let config = Config::load(None)?;
    let project = config.get_project(project_name)?;

    let Some(access_token) = project.access_token.as_ref() else {
        return Ok(None);
    };

    let client = SecretsClient::new(project.project_ref.clone(), access_token.clone());
    let backup = client.backup().await?;

    Ok(Some(backup))
}

/// Restore secrets from backup with interactive prompts or env file
pub async fn restore_secrets(
    backup: &SecretsBackup,
    project_name: &str,
    secrets_file: Option<&Path>,
) -> Result<usize> {
    let config = Config::load(None)?;
    let project = config.get_project(project_name)?;

    let access_token = project
        .access_token
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Project requires access_token for secrets operations"))?;

    if backup.secrets.is_empty() {
        return Ok(0);
    }

    let secrets_to_create = if let Some(file) = secrets_file {
        // Read from env file
        let content = std::fs::read_to_string(file)?;
        let file_secrets = parse_env_file(&content);

        // Match backup secret names with file values
        backup
            .secrets
            .iter()
            .filter_map(|backup_secret| {
                file_secrets
                    .iter()
                    .find(|s| s.name == backup_secret.name && !s.value.is_empty())
                    .cloned()
            })
            .collect::<Vec<_>>()
    } else {
        // Interactive mode
        println!(
            "\n{} Restoring {} secrets (enter values or press Enter to skip)",
            style("🔐").bold(),
            backup.secrets.len()
        );

        let mut secrets = Vec::new();
        for secret in &backup.secrets {
            print!("  {} [press Enter to skip]: ", style(&secret.name).cyan());
            io::stdout().flush()?;

            let value = read_password()?;

            if value.is_empty() {
                println!("    {}", style("(skipped)").dim());
            } else {
                secrets.push(Secret {
                    name: secret.name.clone(),
                    value,
                });
                println!("    {}", style("(set)").green());
            }
        }
        secrets
    };

    if secrets_to_create.is_empty() {
        return Ok(0);
    }

    let client = SecretsClient::new(project.project_ref.clone(), access_token.clone());
    client.create_secrets(&secrets_to_create).await?;

    Ok(secrets_to_create.len())
}
