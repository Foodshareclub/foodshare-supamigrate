use crate::error::{Result, SupamigrateError};
use std::path::Path;
use std::process::Command;
use tracing::{debug, info};

pub struct PgDump {
    db_url: String,
    excluded_schemas: Vec<String>,
    excluded_tables: Vec<String>,
    schema_only: bool,
    data_only: bool,
}

impl PgDump {
    pub fn new(db_url: String) -> Self {
        Self {
            db_url,
            excluded_schemas: Vec::new(),
            excluded_tables: Vec::new(),
            schema_only: false,
            data_only: false,
        }
    }

    pub fn exclude_schemas(mut self, schemas: Vec<String>) -> Self {
        self.excluded_schemas = schemas;
        self
    }

    pub fn exclude_tables(mut self, tables: Vec<String>) -> Self {
        self.excluded_tables = tables;
        self
    }

    pub fn schema_only(mut self, value: bool) -> Self {
        self.schema_only = value;
        self
    }

    pub fn data_only(mut self, value: bool) -> Self {
        self.data_only = value;
        self
    }

    /// Check if pg_dump is available
    pub fn check_available() -> Result<()> {
        let output = Command::new("pg_dump").arg("--version").output();

        match output {
            Ok(o) if o.status.success() => {
                let version = String::from_utf8_lossy(&o.stdout);
                debug!("Found pg_dump: {}", version.trim());
                Ok(())
            }
            _ => Err(SupamigrateError::PgDumpNotFound),
        }
    }

    /// Execute pg_dump and write to file
    pub fn dump_to_file(&self, output_path: &Path) -> Result<()> {
        Self::check_available()?;

        info!("Starting database dump...");

        let mut cmd = Command::new("pg_dump");
        cmd.arg(&self.db_url)
            .arg("--clean")
            .arg("--if-exists")
            .arg("--quote-all-identifiers");

        // Add schema/data only flags
        if self.schema_only {
            cmd.arg("--schema-only");
        }
        if self.data_only {
            cmd.arg("--data-only");
        }

        // Exclude storage.objects data (always)
        cmd.arg("--exclude-table-data=storage.objects");

        // Exclude schemas
        if !self.excluded_schemas.is_empty() {
            let schema_pattern = self.excluded_schemas.join("|");
            cmd.arg(format!("--exclude-schema={}", schema_pattern));
        }

        // Exclude specific tables
        for table in &self.excluded_tables {
            cmd.arg(format!("--exclude-table={}", table));
        }

        // Include all schemas
        cmd.arg("--schema=*");

        // Output to file
        cmd.arg("-f").arg(output_path);

        debug!("Running: {:?}", cmd);

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SupamigrateError::PgDumpFailed(stderr.to_string()));
        }

        info!("Database dump completed: {}", output_path.display());
        Ok(())
    }

    /// Execute pg_dump and return SQL as string
    pub fn dump_to_string(&self) -> Result<String> {
        Self::check_available()?;

        let mut cmd = Command::new("pg_dump");
        cmd.arg(&self.db_url)
            .arg("--clean")
            .arg("--if-exists")
            .arg("--quote-all-identifiers");

        if self.schema_only {
            cmd.arg("--schema-only");
        }
        if self.data_only {
            cmd.arg("--data-only");
        }

        cmd.arg("--exclude-table-data=storage.objects");

        if !self.excluded_schemas.is_empty() {
            let schema_pattern = self.excluded_schemas.join("|");
            cmd.arg(format!("--exclude-schema={}", schema_pattern));
        }

        for table in &self.excluded_tables {
            cmd.arg(format!("--exclude-table={}", table));
        }

        cmd.arg("--schema=*");

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SupamigrateError::PgDumpFailed(stderr.to_string()));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}
