use crate::cli::{StorageArgs, StorageCommands};
use crate::config::Config;
use crate::storage::{StorageClient, StorageTransfer};
use anyhow::Result;
use console::style;

pub async fn run(args: StorageArgs) -> Result<()> {
    match args.command {
        StorageCommands::List { project } => list_buckets(&project).await,
        StorageCommands::Sync {
            from,
            to,
            bucket,
            parallel,
        } => sync_storage(&from, &to, bucket.as_deref(), parallel).await,
        StorageCommands::Download {
            project,
            output,
            bucket,
        } => download_storage(&project, &output, bucket.as_deref()).await,
        StorageCommands::Upload { from, to, bucket } => upload_storage(&from, &to, &bucket).await,
    }
}

async fn list_buckets(project_name: &str) -> Result<()> {
    let config = Config::load(None)?;
    let project = config.get_project(project_name)?;

    let service_key = project.service_key.as_ref().ok_or_else(|| {
        anyhow::anyhow!("Project requires service_key for storage operations")
    })?;

    let client = StorageClient::new(project.api_url(), service_key.clone());
    let buckets = client.list_buckets().await?;

    println!("\n{} Buckets in {}", style("📦").bold(), project_name);
    println!("{:-<50}", "");

    if buckets.is_empty() {
        println!("  No buckets found");
    } else {
        for bucket in buckets {
            let visibility = if bucket.public { "public" } else { "private" };
            println!(
                "  {} {} ({})",
                style("•").cyan(),
                bucket.name,
                visibility
            );
        }
    }

    Ok(())
}

async fn sync_storage(
    from: &str,
    to: &str,
    bucket: Option<&str>,
    parallel: usize,
) -> Result<()> {
    let config = Config::load(None)?;
    let source = config.get_project(from)?;
    let target = config.get_project(to)?;

    let source_key = source.service_key.as_ref().ok_or_else(|| {
        anyhow::anyhow!("Source project requires service_key")
    })?;
    let target_key = target.service_key.as_ref().ok_or_else(|| {
        anyhow::anyhow!("Target project requires service_key")
    })?;

    let source_client = StorageClient::new(source.api_url(), source_key.clone());
    let target_client = StorageClient::new(target.api_url(), target_key.clone());

    println!(
        "\n{} Syncing storage: {} → {}",
        style("📦").bold(),
        from,
        to
    );

    let transfer = StorageTransfer::new(source_client)
        .with_target(target_client)
        .parallel(parallel);

    let stats = if let Some(bucket_name) = bucket {
        let target = config.get_project(to)?;
        let target_key = target.service_key.as_ref().unwrap();
        let target_client = StorageClient::new(target.api_url(), target_key.clone());
        transfer.sync_bucket(bucket_name, &target_client).await?
    } else {
        transfer.sync_all().await?
    };

    println!("\n{} Sync complete: {}", style("✓").green(), stats);
    Ok(())
}

async fn download_storage(
    project_name: &str,
    output: &std::path::Path,
    bucket: Option<&str>,
) -> Result<()> {
    let config = Config::load(None)?;
    let project = config.get_project(project_name)?;

    let service_key = project.service_key.as_ref().ok_or_else(|| {
        anyhow::anyhow!("Project requires service_key")
    })?;

    let client = StorageClient::new(project.api_url(), service_key.clone());

    println!(
        "\n{} Downloading storage from {} to {}",
        style("📦").bold(),
        project_name,
        output.display()
    );

    std::fs::create_dir_all(output)?;

    let transfer = StorageTransfer::new(client)
        .parallel(config.defaults.parallel_transfers);

    let stats = if let Some(bucket_name) = bucket {
        let buckets = transfer.source.list_buckets().await?;
        let bucket = buckets
            .iter()
            .find(|b| b.name == bucket_name)
            .ok_or_else(|| anyhow::anyhow!("Bucket not found: {}", bucket_name))?;
        transfer.download_bucket(bucket, output).await?
    } else {
        transfer.download_all(output).await?
    };

    println!("\n{} Download complete: {}", style("✓").green(), stats);
    Ok(())
}

async fn upload_storage(
    from: &std::path::Path,
    to: &str,
    bucket: &str,
) -> Result<()> {
    use tokio::fs;

    let config = Config::load(None)?;
    let project = config.get_project(to)?;

    let service_key = project.service_key.as_ref().ok_or_else(|| {
        anyhow::anyhow!("Project requires service_key")
    })?;

    let client = StorageClient::new(project.api_url(), service_key.clone());

    println!(
        "\n{} Uploading {} to {}/{}",
        style("📦").bold(),
        from.display(),
        to,
        bucket
    );

    // Create bucket if needed
    client.create_bucket(bucket, false).await?;

    // Upload files
    let mut entries = fs::read_dir(from).await?;
    let mut count = 0;

    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_file() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let data = fs::read(entry.path()).await?;
            
            client.upload(bucket, &file_name, data.into()).await?;
            count += 1;
            println!("  {} {}", style("✓").green(), file_name);
        }
    }

    println!("\n{} Uploaded {} files", style("✓").green(), count);
    Ok(())
}
