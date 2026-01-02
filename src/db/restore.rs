use crate::error::{Result, SupamigrateError};
use std::path::Path;
use std::process::{Command, Stdio};
use tracing::{debug, info};

pub struct PgRestore {
    db_url: String,
}

impl PgRestore {
    pub fn new(db_url: String) -> Self {
        Self { db_url }
    }

    /// Check if psql is available
    pub fn check_available() -> Result<()> {
        let output = Command::new("psql").arg("--version").output();

        match output {
            Ok(o) if o.status.success() => {
                let version = String::from_utf8_lossy(&o.stdout);
                debug!("Found psql: {}", version.trim());
                Ok(())
            }
            _ => Err(SupamigrateError::PsqlNotFound),
        }
    }

    /// Restore from SQL file
    pub fn restore_from_file(&self, input_path: &Path) -> Result<()> {
        Self::check_available()?;

        info!("Starting database restore from {}...", input_path.display());

        let mut cmd = Command::new("psql");
        cmd.arg(&self.db_url)
            .arg("--file")
            .arg(input_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        debug!(
            "Running: psql {} --file {}",
            &self.db_url,
            input_path.display()
        );

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // psql often returns warnings that aren't fatal
            if stderr.contains("ERROR") {
                return Err(SupamigrateError::PsqlFailed(stderr.to_string()));
            }
        }

        info!("Database restore completed");
        Ok(())
    }

    /// Restore from SQL string
    pub fn restore_from_string(&self, sql: &str) -> Result<()> {
        Self::check_available()?;

        info!("Starting database restore...");

        let mut cmd = Command::new("psql");
        cmd.arg(&self.db_url)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn()?;

        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write;
            stdin.write_all(sql.as_bytes())?;
        }

        let output = child.wait_with_output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("ERROR") {
                return Err(SupamigrateError::PsqlFailed(stderr.to_string()));
            }
        }

        info!("Database restore completed");
        Ok(())
    }

    /// Execute a single SQL command
    #[allow(dead_code)]
    pub fn execute(&self, sql: &str) -> Result<String> {
        Self::check_available()?;

        let mut cmd = Command::new("psql");
        cmd.arg(&self.db_url)
            .arg("-c")
            .arg(sql)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SupamigrateError::PsqlFailed(stderr.to_string()));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}
