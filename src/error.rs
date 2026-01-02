use thiserror::Error;

#[derive(Error, Debug)]
pub enum SupamigrateError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("pg_dump not found. Please install PostgreSQL client tools.")]
    PgDumpNotFound,

    #[error("psql not found. Please install PostgreSQL client tools.")]
    PsqlNotFound,

    #[error("pg_dump failed: {0}")]
    PgDumpFailed(String),

    #[error("psql failed: {0}")]
    PsqlFailed(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Bucket not found: {0}")]
    BucketNotFound(String),

    #[error("Edge Functions error: {0}")]
    Functions(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("Operation cancelled by user")]
    Cancelled,

    #[error("Backup not found at: {0}")]
    BackupNotFound(String),

    #[error("Invalid backup format: {0}")]
    InvalidBackup(String),
}

pub type Result<T> = std::result::Result<T, SupamigrateError>;
