use crate::error::{Result, SupamigrateError};
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use tracing::debug;

/// A secret stored in Supabase Vault
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultSecret {
    pub id: String,
    pub name: String,
    pub secret: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Backup structure for vault secrets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultBackup {
    pub secrets: Vec<VaultSecret>,
    pub exported_at: String,
}

/// Client for interacting with Supabase Vault via SQL
pub struct VaultClient {
    db_url: String,
}

impl VaultClient {
    pub fn new(db_url: String) -> Self {
        Self { db_url }
    }

    /// Execute a SQL query and return the output
    fn query(&self, sql: &str) -> Result<String> {
        let mut cmd = Command::new("psql");
        cmd.arg(&self.db_url)
            .arg("-t") // Tuples only (no headers)
            .arg("-A") // Unaligned output
            .arg("-c")
            .arg(sql)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        debug!("Executing vault query: {}", sql);

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SupamigrateError::Vault(format!("Query failed: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Execute a SQL query that returns JSON
    fn query_json<T: for<'de> Deserialize<'de>>(&self, sql: &str) -> Result<T> {
        let output = self.query(sql)?;
        if output.is_empty() {
            return Err(SupamigrateError::Vault("Empty response".to_string()));
        }
        serde_json::from_str(&output).map_err(|e| {
            SupamigrateError::Vault(format!("Failed to parse JSON: {} - Output: {}", e, output))
        })
    }

    /// Check if the vault extension is enabled in the database
    pub fn is_vault_enabled(&self) -> Result<bool> {
        let sql = "SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'supabase_vault')";
        let result = self.query(sql)?;
        Ok(result == "t" || result == "true")
    }

    /// List all secrets from the vault (with decrypted values)
    pub fn list_secrets(&self) -> Result<Vec<VaultSecret>> {
        // First check if vault is enabled
        if !self.is_vault_enabled()? {
            return Ok(vec![]);
        }

        let sql = r"
            SELECT COALESCE(
                json_agg(
                    json_build_object(
                        'id', id::text,
                        'name', name,
                        'secret', secret,
                        'description', description,
                        'created_at', created_at::text,
                        'updated_at', updated_at::text
                    )
                ),
                '[]'::json
            )::text
            FROM vault.decrypted_secrets
        ";

        self.query_json(sql)
    }

    /// Create a new secret in the vault
    pub fn create_secret(
        &self,
        name: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<String> {
        // Escape single quotes in values
        let name_escaped = name.replace('\'', "''");
        let value_escaped = value.replace('\'', "''");

        let sql = if let Some(desc) = description {
            let desc_escaped = desc.replace('\'', "''");
            format!(
                "SELECT vault.create_secret('{}', '{}', '{}')::text",
                name_escaped, value_escaped, desc_escaped
            )
        } else {
            format!(
                "SELECT vault.create_secret('{}', '{}')::text",
                name_escaped, value_escaped
            )
        };

        self.query(&sql)
    }

    /// Update an existing secret
    #[allow(dead_code)]
    pub fn update_secret(
        &self,
        id: &str,
        new_value: &str,
        new_name: Option<&str>,
        new_description: Option<&str>,
    ) -> Result<()> {
        let value_escaped = new_value.replace('\'', "''");
        let name_part = new_name.map_or_else(
            || "NULL".to_string(),
            |n| format!("'{}'", n.replace('\'', "''")),
        );
        let desc_part = new_description.map_or_else(
            || "NULL".to_string(),
            |d| format!("'{}'", d.replace('\'', "''")),
        );

        let sql = format!(
            "SELECT vault.update_secret('{}', '{}', {}, {})",
            id, value_escaped, name_part, desc_part
        );

        self.query(&sql)?;
        Ok(())
    }

    /// Backup all vault secrets
    pub fn backup(&self) -> Result<VaultBackup> {
        let secrets = self.list_secrets()?;
        Ok(VaultBackup {
            secrets,
            exported_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Restore secrets from a backup
    pub fn restore(&self, backup: &VaultBackup) -> Result<usize> {
        let mut count = 0;

        for secret in &backup.secrets {
            // Check if secret with same name exists
            let check_sql = format!(
                "SELECT COUNT(*) FROM vault.decrypted_secrets WHERE name = '{}'",
                secret.name.replace('\'', "''")
            );
            let exists = self.query(&check_sql)?.parse::<i32>().unwrap_or(0) > 0;

            if exists {
                debug!("Secret '{}' already exists, skipping", secret.name);
                continue;
            }

            self.create_secret(&secret.name, &secret.secret, secret.description.as_deref())?;
            count += 1;
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_backup_serialization() {
        let backup = VaultBackup {
            secrets: vec![VaultSecret {
                id: "123".to_string(),
                name: "API_KEY".to_string(),
                secret: "secret_value".to_string(),
                description: Some("Test key".to_string()),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                updated_at: "2024-01-01T00:00:00Z".to_string(),
            }],
            exported_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string_pretty(&backup).unwrap();
        assert!(json.contains("API_KEY"));
        assert!(json.contains("secret_value"));
    }
}
