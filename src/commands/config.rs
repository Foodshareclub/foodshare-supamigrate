use crate::cli::{ConfigArgs, ConfigCommands};
use crate::config::{generate_sample_config, Config, ProjectConfig};
use anyhow::Result;
use console::style;
use std::fs;

pub fn run(args: ConfigArgs) -> Result<()> {
    match args.command {
        ConfigCommands::Init { output } => init_config(&output),
        ConfigCommands::Add {
            alias,
            project_ref,
            db_password,
            service_key,
            access_token,
        } => add_project(&alias, &project_ref, &db_password, service_key, access_token),
        ConfigCommands::List => list_projects(),
        ConfigCommands::Show => show_config(),
    }
}

fn init_config(output: &std::path::Path) -> Result<()> {
    if output.exists() {
        println!(
            "{} Config file already exists: {}",
            style("⚠️").yellow(),
            output.display()
        );
        return Ok(());
    }

    let sample = generate_sample_config();
    fs::write(output, sample)?;

    println!(
        "{} Created config file: {}",
        style("✓").green(),
        output.display()
    );
    println!("\nEdit the file to add your Supabase project credentials.");

    Ok(())
}

fn add_project(
    alias: &str,
    project_ref: &str,
    db_password: &str,
    service_key: Option<String>,
    access_token: Option<String>,
) -> Result<()> {
    let config_path = std::path::Path::new("./supamigrate.toml");

    let mut config = if config_path.exists() {
        Config::load(Some(config_path))?
    } else {
        Config::default()
    };

    let project = ProjectConfig {
        project_ref: project_ref.to_string(),
        db_password: db_password.to_string(),
        service_key,
        db_host: None,
        db_port: None,
        api_url: None,
        access_token,
    };

    config.add_project(alias.to_string(), project);
    config.save(config_path)?;

    println!(
        "{} Added project '{}' ({})",
        style("✓").green(),
        alias,
        project_ref
    );

    Ok(())
}

fn list_projects() -> Result<()> {
    let config = Config::load(None)?;

    println!("\n{} Configured Projects", style("📋").bold());
    println!("{:-<50}", "");

    if config.projects.is_empty() {
        println!("  No projects configured");
        println!("\n  Run 'supamigrate config init' to create a config file");
    } else {
        for (alias, project) in &config.projects {
            let storage = if project.has_storage_access() {
                style("✓").green()
            } else {
                style("✗").red()
            };
            let secrets = if project.has_secrets_access() {
                style("✓").green()
            } else {
                style("✗").red()
            };
            println!(
                "  {} {} → {} (storage: {}, secrets: {})",
                style("•").cyan(),
                alias,
                project.project_ref,
                storage,
                secrets
            );
        }
    }

    Ok(())
}

fn show_config() -> Result<()> {
    let config = Config::load(None)?;

    println!("\n{} Current Configuration", style("⚙️").bold());
    println!("{:-<50}", "");

    println!("\nDefaults:");
    println!(
        "  Parallel transfers: {}",
        config.defaults.parallel_transfers
    );
    println!("  Compress backups: {}", config.defaults.compress_backups);
    println!("  Excluded schemas:");
    for schema in &config.defaults.excluded_schemas {
        println!("    - {}", schema);
    }

    println!("\nProjects:");
    for (alias, project) in &config.projects {
        println!("  [{}]", alias);
        println!("    project_ref: {}", project.project_ref);
        println!("    db_password: ****");
        println!(
            "    service_key: {}",
            if project.service_key.is_some() {
                "****"
            } else {
                "(not set)"
            }
        );
        println!(
            "    access_token: {}",
            if project.access_token.is_some() {
                "****"
            } else {
                "(not set)"
            }
        );
        if let Some(host) = &project.db_host {
            println!("    db_host: {}", host);
        }
        if let Some(port) = &project.db_port {
            println!("    db_port: {}", port);
        }
    }

    Ok(())
}
