use crate::error::{Result, SupamigrateError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

const DEFAULT_CONFIG_PATHS: &[&str] = &[
    "./supamigrate.toml",
    "~/.config/supamigrate/config.toml",
    "~/.supamigrate.toml",
];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub projects: HashMap<String, ProjectConfig>,

    #[serde(default)]
    pub defaults: DefaultsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Supabase project reference (e.g., "abcdefghijklmnop")
    pub project_ref: String,

    /// Database password
    pub db_password: String,

    /// Service role key (required for storage operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_key: Option<String>,

    /// Custom database host (defaults to db.{project_ref}.supabase.co)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_host: Option<String>,

    /// Custom database port (defaults to 5432)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_port: Option<u16>,

    /// Custom API URL (defaults to https://{project_ref}.supabase.co)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_url: Option<String>,

    /// Personal access token (required for secrets operations)
    /// Generate at: <https://supabase.com/dashboard/account/tokens>
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DefaultsConfig {
    /// Default number of parallel storage transfers
    #[serde(default = "default_parallel")]
    pub parallel_transfers: usize,

    /// Default schemas to exclude
    #[serde(default = "default_excluded_schemas")]
    pub excluded_schemas: Vec<String>,

    /// Compress backups by default
    #[serde(default = "default_compress")]
    pub compress_backups: bool,
}

fn default_parallel() -> usize {
    4
}

fn default_compress() -> bool {
    true
}

fn default_excluded_schemas() -> Vec<String> {
    vec![
        "extensions".to_string(),
        "graphql".to_string(),
        "graphql_public".to_string(),
        "net".to_string(),
        "pgbouncer".to_string(),
        "pgsodium".to_string(),
        "pgsodium_masks".to_string(),
        "realtime".to_string(),
        "supabase_functions".to_string(),
        "storage".to_string(),
        "pg_*".to_string(),
        "information_schema".to_string(),
    ]
}

impl Config {
    /// Load config from file or default locations
    pub fn load(path: Option<&Path>) -> Result<Self> {
        if let Some(p) = path {
            return Self::load_from_path(p);
        }

        // Try default locations
        for default_path in DEFAULT_CONFIG_PATHS {
            let expanded = shellexpand::tilde(default_path);
            let path = Path::new(expanded.as_ref());
            if path.exists() {
                return Self::load_from_path(path);
            }
        }

        // Return default config if no file found
        Ok(Self::default())
    }

    fn load_from_path(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save config to file
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get project config by alias or project_ref
    pub fn get_project(&self, name: &str) -> Result<&ProjectConfig> {
        // First try as alias
        if let Some(project) = self.projects.get(name) {
            return Ok(project);
        }

        // Then try as project_ref
        for project in self.projects.values() {
            if project.project_ref == name {
                return Ok(project);
            }
        }

        Err(SupamigrateError::ProjectNotFound(name.to_string()))
    }

    /// Add a project to config
    pub fn add_project(&mut self, alias: String, project: ProjectConfig) {
        self.projects.insert(alias, project);
    }
}

impl ProjectConfig {
    /// Get the database connection URL
    pub fn db_url(&self) -> String {
        let host = self
            .db_host
            .clone()
            .unwrap_or_else(|| format!("db.{}.supabase.co", self.project_ref));
        let port = self.db_port.unwrap_or(5432);

        format!(
            "postgres://postgres:{}@{}:{}/postgres",
            self.db_password, host, port
        )
    }

    /// Get the Supabase API URL
    pub fn api_url(&self) -> String {
        self.api_url
            .clone()
            .unwrap_or_else(|| format!("https://{}.supabase.co", self.project_ref))
    }

    /// Check if storage operations are available
    pub fn has_storage_access(&self) -> bool {
        self.service_key.is_some()
    }

    /// Check if secrets operations are available
    pub fn has_secrets_access(&self) -> bool {
        self.access_token.is_some()
    }
}

/// Generate a sample config file
pub fn generate_sample_config() -> String {
    r#"# Supamigrate Configuration
# https://github.com/foodshare-club/supamigrate

# Define your Supabase projects here
[projects.production]
project_ref = "your-prod-project-ref"
db_password = "your-db-password"
service_key = "your-service-role-key"  # Optional, needed for storage
access_token = "sbp_xxx"  # Optional, needed for secrets (https://supabase.com/dashboard/account/tokens)

[projects.staging]
project_ref = "your-staging-project-ref"
db_password = "your-db-password"
service_key = "your-service-role-key"
access_token = "sbp_xxx"

# Default settings
[defaults]
parallel_transfers = 4
compress_backups = true
excluded_schemas = [
    "extensions",
    "graphql",
    "graphql_public",
    "net",
    "pgbouncer",
    "pgsodium",
    "pgsodium_masks",
    "realtime",
    "supabase_functions",
    "storage",
    "pg_*",
    "information_schema"
]
"#
    .to_string()
}
