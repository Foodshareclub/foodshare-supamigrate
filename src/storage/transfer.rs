use crate::error::Result;
use crate::storage::client::{Bucket, StorageClient, StorageObject};
use futures::stream::{self, StreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tracing::info;

pub struct StorageTransfer {
    pub source: StorageClient,
    target: Option<StorageClient>,
    parallel: usize,
}

impl StorageTransfer {
    pub fn new(source: StorageClient) -> Self {
        Self {
            source,
            target: None,
            parallel: 4,
        }
    }

    pub fn with_target(mut self, target: StorageClient) -> Self {
        self.target = Some(target);
        self
    }

    pub fn parallel(mut self, count: usize) -> Self {
        self.parallel = count;
        self
    }

    /// Sync all buckets from source to target
    pub async fn sync_all(&self) -> Result<SyncStats> {
        let target = self
            .target
            .as_ref()
            .expect("Target client required for sync");

        let buckets = self.source.list_buckets().await?;
        info!("Found {} buckets to sync", buckets.len());

        let mut stats = SyncStats::default();

        for bucket in buckets {
            let bucket_stats = self.sync_bucket(&bucket.name, target).await?;
            stats.buckets += 1;
            stats.objects += bucket_stats.objects;
            stats.bytes += bucket_stats.bytes;
        }

        Ok(stats)
    }

    /// Sync a specific bucket
    pub async fn sync_bucket(&self, bucket_name: &str, target: &StorageClient) -> Result<SyncStats> {
        info!("Syncing bucket: {}", bucket_name);

        // Get bucket info and create on target
        let buckets = self.source.list_buckets().await?;
        let bucket = buckets
            .iter()
            .find(|b| b.name == bucket_name)
            .ok_or_else(|| crate::error::SupamigrateError::BucketNotFound(bucket_name.to_string()))?;

        target.create_bucket(&bucket.name, bucket.public).await?;

        // List and transfer objects
        let objects = self.source.list_objects(bucket_name, None).await?;
        self.transfer_objects(bucket_name, &objects, target).await
    }

    /// Transfer objects with progress
    async fn transfer_objects(
        &self,
        bucket: &str,
        objects: &[StorageObject],
        target: &StorageClient,
    ) -> Result<SyncStats> {
        let multi = MultiProgress::new();
        let pb = multi.add(ProgressBar::new(objects.len() as u64));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message(format!("Syncing {}", bucket));

        let source = Arc::new(self.source.clone());
        let target = Arc::new(target.clone());
        let bucket = bucket.to_string();

        let mut stats = SyncStats::default();

        let results: Vec<Result<usize>> = stream::iter(objects.iter())
            .map(|obj| {
                let source = Arc::clone(&source);
                let target = Arc::clone(&target);
                let bucket = bucket.clone();
                let name = obj.name.clone();
                let pb = pb.clone();

                async move {
                    let data = source.download(&bucket, &name).await?;
                    let size = data.len();
                    target.upload(&bucket, &name, data).await?;
                    pb.inc(1);
                    Ok(size)
                }
            })
            .buffer_unordered(self.parallel)
            .collect()
            .await;

        pb.finish_with_message("Done");

        for result in results {
            match result {
                Ok(size) => {
                    stats.objects += 1;
                    stats.bytes += size;
                }
                Err(e) => {
                    stats.errors += 1;
                    tracing::warn!("Transfer error: {}", e);
                }
            }
        }

        Ok(stats)
    }

    /// Download all buckets to local directory
    pub async fn download_all(&self, output_dir: &Path) -> Result<SyncStats> {
        let buckets = self.source.list_buckets().await?;
        info!("Downloading {} buckets", buckets.len());

        let mut stats = SyncStats::default();

        for bucket in buckets {
            let bucket_stats = self.download_bucket(&bucket, output_dir).await?;
            stats.buckets += 1;
            stats.objects += bucket_stats.objects;
            stats.bytes += bucket_stats.bytes;
        }

        Ok(stats)
    }

    /// Download a bucket to local directory
    pub async fn download_bucket(&self, bucket: &Bucket, output_dir: &Path) -> Result<SyncStats> {
        let bucket_dir = output_dir.join(&bucket.name);
        fs::create_dir_all(&bucket_dir).await?;

        let objects = self.source.list_objects(&bucket.name, None).await?;
        info!("Downloading {} objects from {}", objects.len(), bucket.name);

        let multi = MultiProgress::new();
        let pb = multi.add(ProgressBar::new(objects.len() as u64));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message(format!("Downloading {}", bucket.name));

        let source = Arc::new(self.source.clone());
        let bucket_name = bucket.name.clone();

        let mut stats = SyncStats::default();

        let results: Vec<Result<usize>> = stream::iter(objects.iter())
            .map(|obj| {
                let source = Arc::clone(&source);
                let bucket_name = bucket_name.clone();
                let bucket_dir = bucket_dir.clone();
                let name = obj.name.clone();
                let pb = pb.clone();

                async move {
                    let data = source.download(&bucket_name, &name).await?;
                    let size = data.len();

                    let file_path = bucket_dir.join(&name);
                    if let Some(parent) = file_path.parent() {
                        fs::create_dir_all(parent).await?;
                    }
                    fs::write(&file_path, &data).await?;

                    pb.inc(1);
                    Ok(size)
                }
            })
            .buffer_unordered(self.parallel)
            .collect()
            .await;

        pb.finish_with_message("Done");

        for result in results {
            match result {
                Ok(size) => {
                    stats.objects += 1;
                    stats.bytes += size;
                }
                Err(e) => {
                    stats.errors += 1;
                    tracing::warn!("Download error: {}", e);
                }
            }
        }

        Ok(stats)
    }
}

#[derive(Debug, Default)]
pub struct SyncStats {
    pub buckets: usize,
    pub objects: usize,
    pub bytes: usize,
    pub errors: usize,
}

impl std::fmt::Display for SyncStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} buckets, {} objects, {} bytes transferred",
            self.buckets,
            self.objects,
            human_bytes(self.bytes)
        )?;
        if self.errors > 0 {
            write!(f, " ({} errors)", self.errors)?;
        }
        Ok(())
    }
}

fn human_bytes(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;
    const GB: usize = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
