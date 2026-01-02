use crate::error::{Result, SupamigrateError};
use bytes::Bytes;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Clone)]
pub struct StorageClient {
    client: Client,
    api_url: String,
    service_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bucket {
    pub id: String,
    pub name: String,
    pub public: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageObject {
    pub name: String,
    pub id: Option<String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct CreateBucketRequest {
    name: String,
    public: bool,
}

impl StorageClient {
    pub fn new(api_url: String, service_key: String) -> Self {
        Self {
            client: Client::new(),
            api_url,
            service_key,
        }
    }

    fn storage_url(&self) -> String {
        format!("{}/storage/v1", self.api_url)
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.service_key)
    }

    /// List all buckets
    pub async fn list_buckets(&self) -> Result<Vec<Bucket>> {
        let url = format!("{}/bucket", self.storage_url());
        debug!("Listing buckets: {}", url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("apikey", &self.service_key)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SupamigrateError::Storage(format!(
                "Failed to list buckets: {} - {}",
                status, body
            )));
        }

        let buckets: Vec<Bucket> = response.json().await?;
        Ok(buckets)
    }

    /// Create a bucket
    pub async fn create_bucket(&self, name: &str, public: bool) -> Result<()> {
        let url = format!("{}/bucket", self.storage_url());
        debug!("Creating bucket: {}", name);

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("apikey", &self.service_key)
            .json(&CreateBucketRequest {
                name: name.to_string(),
                public,
            })
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            // Ignore "already exists" errors
            if !body.contains("already exists") {
                return Err(SupamigrateError::Storage(format!(
                    "Failed to create bucket '{}': {} - {}",
                    name, status, body
                )));
            }
        }

        Ok(())
    }

    /// List objects in a bucket
    pub async fn list_objects(
        &self,
        bucket: &str,
        prefix: Option<&str>,
    ) -> Result<Vec<StorageObject>> {
        let url = format!("{}/object/list/{}", self.storage_url(), bucket);
        debug!("Listing objects in bucket: {}", bucket);

        let mut body = serde_json::json!({
            "limit": 1000,
            "offset": 0,
        });

        if let Some(p) = prefix {
            body["prefix"] = serde_json::json!(p);
        }

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("apikey", &self.service_key)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SupamigrateError::Storage(format!(
                "Failed to list objects in '{}': {} - {}",
                bucket, status, body
            )));
        }

        let objects: Vec<StorageObject> = response.json().await?;
        Ok(objects)
    }

    /// Download an object
    pub async fn download(&self, bucket: &str, path: &str) -> Result<Bytes> {
        let url = format!("{}/object/{}/{}", self.storage_url(), bucket, path);
        debug!("Downloading: {}/{}", bucket, path);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("apikey", &self.service_key)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SupamigrateError::Storage(format!(
                "Failed to download '{}/{}': {} - {}",
                bucket, path, status, body
            )));
        }

        let bytes = response.bytes().await?;
        Ok(bytes)
    }

    /// Upload an object
    pub async fn upload(&self, bucket: &str, path: &str, data: Bytes) -> Result<()> {
        let url = format!("{}/object/{}/{}", self.storage_url(), bucket, path);
        debug!("Uploading: {}/{}", bucket, path);

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("apikey", &self.service_key)
            .header("Content-Type", "application/octet-stream")
            .body(data)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SupamigrateError::Storage(format!(
                "Failed to upload '{}/{}': {} - {}",
                bucket, path, status, body
            )));
        }

        Ok(())
    }
}
