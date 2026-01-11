use crate::error::{Result, SupamigrateError};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, info, warn};

pub struct PgDump {
    db_url: String,
    binary_path: PathBuf,
    excluded_schemas: Vec<String>,
    excluded_tables: Vec<String>,
    schema_only: bool,
    data_only: bool,
}

/// Query remote server for PostgreSQL major version
fn get_server_version(db_url: &str) -> Option<u32> {
    let output = Command::new("psql")
        .arg(db_url)
        .arg("-t") // tuples only
        .arg("-A") // unaligned
        .arg("-c")
        .arg("SHOW server_version_num")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    // server_version_num returns e.g. "150001" for 15.0.1
    // Major version = first 2 digits for PG >= 10
    let version_str = String::from_utf8_lossy(&output.stdout);
    let version_num: u32 = version_str.trim().parse().ok()?;
    Some(version_num / 10000) // 150001 -> 15
}

/// Find pg_dump binary compatible with server version
fn find_compatible_pg_dump(server_major: u32) -> PathBuf {
    // Check versions from exact match up to +3 (pg_dump is forward-compatible)
    let versions_to_try: Vec<u32> = (server_major..=server_major + 3).collect();

    for version in versions_to_try {
        let paths = if cfg!(target_os = "macos") {
            vec![
                // Apple Silicon Homebrew
                format!("/opt/homebrew/opt/postgresql@{}/bin/pg_dump", version),
                // Intel Homebrew
                format!("/usr/local/opt/postgresql@{}/bin/pg_dump", version),
                // Postgres.app
                format!(
                    "/Applications/Postgres.app/Contents/Versions/{}/bin/pg_dump",
                    version
                ),
            ]
        } else {
            // Linux paths
            vec![
                format!("/usr/lib/postgresql/{}/bin/pg_dump", version),
                format!("/usr/pgsql-{}/bin/pg_dump", version),
            ]
        };

        for path in paths {
            if Path::new(&path).exists() {
                debug!("Found compatible pg_dump v{} at: {}", version, path);
                return PathBuf::from(path);
            }
        }
    }

    // Fall back to PATH
    debug!("No version-specific pg_dump found, using PATH");
    PathBuf::from("pg_dump")
}

impl PgDump {
    pub fn new(db_url: String) -> Self {
        // Try to auto-detect compatible pg_dump
        let binary_path = match get_server_version(&db_url) {
            Some(major) => {
                info!("Detected PostgreSQL server version: {}", major);
                find_compatible_pg_dump(major)
            }
            None => {
                warn!("Could not detect server version, using pg_dump from PATH");
                PathBuf::from("pg_dump")
            }
        };

        Self {
            db_url,
            binary_path,
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
    fn check_available(&self) -> Result<()> {
        let output = Command::new(&self.binary_path).arg("--version").output();

        match output {
            Ok(o) if o.status.success() => {
                let version = String::from_utf8_lossy(&o.stdout);
                debug!(
                    "Using pg_dump: {} ({})",
                    self.binary_path.display(),
                    version.trim()
                );
                Ok(())
            }
            _ => Err(SupamigrateError::PgDumpNotFound),
        }
    }

    /// Execute pg_dump and write to file
    #[allow(dead_code)]
    pub fn dump_to_file(&self, output_path: &Path) -> Result<()> {
        self.check_available()?;

        info!("Starting database dump...");

        let mut cmd = Command::new(&self.binary_path);
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
        self.check_available()?;

        let mut cmd = Command::new(&self.binary_path);
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
