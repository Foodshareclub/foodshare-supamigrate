use crate::error::{Result, SupamigrateError};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

const SUPABASE_API_URL: &str = "https://api.supabase.com";

#[derive(Debug, Clone)]
pub struct SecretsClient {
    client: Client,
    project_ref: String,
    access_token: String,
}

/// Metadata for a secret (values are never exposed by the API)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretMetadata {
    pub name: String,
}

/// Secret with value for creating/updating
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Secret {
    pub name: String,
    pub value: String,
}

/// Backup of secret names (values cannot be backed up)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsBackup {
    pub secrets: Vec<SecretMetadata>,
    #[serde(default)]
    pub note: String,
}

impl SecretsClient {
    pub fn new(project_ref: String, access_token: String) -> Self {
        Self {
            client: Client::new(),
            project_ref,
            access_token,
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.access_token)
    }

    /// List all secrets (names only, values are not exposed)
    pub async fn list_secrets(&self) -> Result<Vec<SecretMetadata>> {
        let url = format!(
            "{}/v1/projects/{}/secrets",
            SUPABASE_API_URL, self.project_ref
        );
        debug!("Listing secrets: {}", url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SupamigrateError::Secrets(format!(
                "Failed to list secrets: {} - {}",
                status, body
            )));
        }

        let secrets: Vec<SecretMetadata> = response.json().await?;
        Ok(secrets)
    }

    /// Create or update multiple secrets
    pub async fn create_secrets(&self, secrets: &[Secret]) -> Result<()> {
        if secrets.is_empty() {
            return Ok(());
        }

        let url = format!(
            "{}/v1/projects/{}/secrets",
            SUPABASE_API_URL, self.project_ref
        );
        debug!("Creating {} secrets", secrets.len());

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(secrets)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SupamigrateError::Secrets(format!(
                "Failed to create secrets: {} - {}",
                status, body
            )));
        }

        Ok(())
    }

    /// Delete multiple secrets by name
    #[allow(dead_code)]
    pub async fn delete_secrets(&self, names: &[String]) -> Result<()> {
        if names.is_empty() {
            return Ok(());
        }

        let url = format!(
            "{}/v1/projects/{}/secrets",
            SUPABASE_API_URL, self.project_ref
        );
        debug!("Deleting {} secrets", names.len());

        let response = self
            .client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(names)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SupamigrateError::Secrets(format!(
                "Failed to delete secrets: {} - {}",
                status, body
            )));
        }

        Ok(())
    }

    /// Backup all secret names (values cannot be backed up for security)
    pub async fn backup(&self) -> Result<SecretsBackup> {
        let secrets = self.list_secrets().await?;
        Ok(SecretsBackup {
            secrets,
            note:
                "Secret values cannot be backed up via API. You must provide values during restore."
                    .to_string(),
        })
    }
}

/// Parse secrets from an env file format (NAME=value)
pub fn parse_env_file(content: &str) -> Vec<Secret> {
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            // Split on first '='
            let (name, value) = line.split_once('=')?;
            let name = name.trim().to_string();
            let value = value.trim().to_string();
            // Remove surrounding quotes if present
            let value = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .map(String::from)
                .unwrap_or(value);
            Some(Secret { name, value })
        })
        .collect()
}

/// Generate an env file template from secret names
pub fn generate_env_template(secrets: &[SecretMetadata]) -> String {
    use std::fmt::Write;
    let mut output = String::from("# Secrets template generated by supamigrate\n");
    output.push_str("# Fill in the values below and use with: supamigrate secrets import --file <this-file>\n\n");
    for secret in secrets {
        let _ = writeln!(output, "{}=", secret.name);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_env_file() {
        let content = r#"
# Comment line
API_KEY=secret123
DATABASE_URL="postgres://localhost/db"
EMPTY_VALUE=

ANOTHER_KEY=value with spaces
"#;
        let secrets = parse_env_file(content);
        assert_eq!(secrets.len(), 4);
        assert_eq!(secrets[0].name, "API_KEY");
        assert_eq!(secrets[0].value, "secret123");
        assert_eq!(secrets[1].name, "DATABASE_URL");
        assert_eq!(secrets[1].value, "postgres://localhost/db");
        assert_eq!(secrets[2].name, "EMPTY_VALUE");
        assert_eq!(secrets[2].value, "");
        assert_eq!(secrets[3].name, "ANOTHER_KEY");
        assert_eq!(secrets[3].value, "value with spaces");
    }

    #[test]
    fn test_generate_env_template() {
        let secrets = vec![
            SecretMetadata {
                name: "API_KEY".to_string(),
            },
            SecretMetadata {
                name: "DATABASE_URL".to_string(),
            },
        ];
        let template = generate_env_template(&secrets);
        assert!(template.contains("API_KEY="));
        assert!(template.contains("DATABASE_URL="));
    }
}
