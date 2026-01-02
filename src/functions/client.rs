use crate::error::{Result, SupamigrateError};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

const SUPABASE_API_URL: &str = "https://api.supabase.com";

#[derive(Debug, Clone)]
pub struct FunctionsClient {
    client: Client,
    project_ref: String,
    service_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeFunction {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub status: String,
    pub version: i32,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub verify_jwt: bool,
    #[serde(default)]
    pub import_map: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entrypoint_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub import_map_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeFunctionBody {
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(default)]
    pub verify_jwt: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entrypoint_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub import_map_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionBackup {
    pub slug: String,
    pub name: String,
    pub verify_jwt: bool,
    pub entrypoint_path: Option<String>,
    pub import_map_path: Option<String>,
    pub files: Vec<FunctionFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionFile {
    pub name: String,
    pub content: String,
}

impl FunctionsClient {
    pub fn new(project_ref: String, service_key: String) -> Self {
        Self {
            client: Client::new(),
            project_ref,
            service_key,
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.service_key)
    }

    /// List all edge functions
    pub async fn list_functions(&self) -> Result<Vec<EdgeFunction>> {
        let url = format!(
            "{}/v1/projects/{}/functions",
            SUPABASE_API_URL, self.project_ref
        );
        debug!("Listing edge functions: {}", url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SupamigrateError::Functions(format!(
                "Failed to list functions: {} - {}",
                status, body
            )));
        }

        let functions: Vec<EdgeFunction> = response.json().await?;
        Ok(functions)
    }

    /// Get function details including source code
    #[allow(dead_code)]
    pub async fn get_function(&self, slug: &str) -> Result<EdgeFunctionBody> {
        let url = format!(
            "{}/v1/projects/{}/functions/{}/body",
            SUPABASE_API_URL, self.project_ref, slug
        );
        debug!("Getting function body: {}", url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SupamigrateError::Functions(format!(
                "Failed to get function '{}': {} - {}",
                slug, status, body
            )));
        }

        let function: EdgeFunctionBody = response.json().await?;
        Ok(function)
    }

    /// Download function source as a tarball and extract files
    pub async fn download_function_source(&self, slug: &str) -> Result<Vec<FunctionFile>> {
        let url = format!(
            "{}/v1/projects/{}/functions/{}/body",
            SUPABASE_API_URL, self.project_ref, slug
        );
        debug!("Downloading function source: {}", url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("Accept", "application/octet-stream")
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SupamigrateError::Functions(format!(
                "Failed to download function '{}': {} - {}",
                slug, status, body
            )));
        }

        // Check content type - might be JSON or tarball
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if content_type.contains("application/json") {
            // Single file function returned as JSON
            let body: EdgeFunctionBody = response.json().await?;
            if let Some(source) = body.body {
                return Ok(vec![FunctionFile {
                    name: body
                        .entrypoint_path
                        .unwrap_or_else(|| "index.ts".to_string()),
                    content: source,
                }]);
            }
            return Ok(vec![]);
        }

        // Tarball - extract files
        let bytes = response.bytes().await?;
        let files = extract_tarball(&bytes)?;
        Ok(files)
    }

    /// Create or update an edge function
    pub async fn deploy_function(&self, backup: &FunctionBackup) -> Result<()> {
        // First check if function exists
        let exists = self
            .list_functions()
            .await?
            .iter()
            .any(|f| f.slug == backup.slug);

        let url = if exists {
            format!(
                "{}/v1/projects/{}/functions/{}",
                SUPABASE_API_URL, self.project_ref, backup.slug
            )
        } else {
            format!(
                "{}/v1/projects/{}/functions",
                SUPABASE_API_URL, self.project_ref
            )
        };

        debug!("Deploying function '{}' (exists: {})", backup.slug, exists);

        // Build multipart form with files
        let mut form = reqwest::multipart::Form::new();

        // Add metadata
        let metadata = serde_json::json!({
            "name": backup.name,
            "slug": backup.slug,
            "verify_jwt": backup.verify_jwt,
            "entrypoint_path": backup.entrypoint_path,
            "import_map_path": backup.import_map_path,
        });
        form = form.text("metadata", metadata.to_string());

        // Add files
        for file in &backup.files {
            form = form.text(file.name.clone(), file.content.clone());
        }

        let request = if exists {
            self.client.patch(&url)
        } else {
            self.client.post(&url)
        };

        let response = request
            .header("Authorization", self.auth_header())
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SupamigrateError::Functions(format!(
                "Failed to deploy function '{}': {} - {}",
                backup.slug, status, body
            )));
        }

        Ok(())
    }

    /// Backup all edge functions
    pub async fn backup_all(&self) -> Result<Vec<FunctionBackup>> {
        let functions = self.list_functions().await?;
        let mut backups = Vec::new();

        for func in functions {
            debug!("Backing up function: {}", func.slug);
            let files = self.download_function_source(&func.slug).await?;

            backups.push(FunctionBackup {
                slug: func.slug,
                name: func.name,
                verify_jwt: func.verify_jwt,
                entrypoint_path: func.entrypoint_path,
                import_map_path: func.import_map_path,
                files,
            });
        }

        Ok(backups)
    }
}

/// Extract files from a gzipped tarball
fn extract_tarball(data: &[u8]) -> Result<Vec<FunctionFile>> {
    use flate2::read::GzDecoder;
    use std::io::Read;

    let mut files = Vec::new();

    // Try to decompress as gzip first
    let decoder = GzDecoder::new(data);
    let mut archive = tar::Archive::new(decoder);

    match archive.entries() {
        Ok(entries) => {
            for entry in entries {
                let mut entry = entry?;
                let path = entry.path()?.to_string_lossy().to_string();

                // Skip directories
                if entry.header().entry_type().is_dir() {
                    continue;
                }

                let mut content = String::new();
                entry.read_to_string(&mut content)?;

                files.push(FunctionFile {
                    name: path,
                    content,
                });
            }
        }
        Err(_) => {
            // Not a tarball, try as plain text
            let content = String::from_utf8_lossy(data).to_string();
            files.push(FunctionFile {
                name: "index.ts".to_string(),
                content,
            });
        }
    }

    Ok(files)
}
