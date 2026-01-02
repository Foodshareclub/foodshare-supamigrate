use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "supamigrate",
    author,
    version,
    about = "CLI tool for migrating Supabase projects",
    long_about = "Migrate database schema, data, and storage between Supabase projects.\n\n\
                  Supports full migrations, schema-only, data-only, and storage transfers."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Config file path
    #[arg(short, long, global = true, env = "SUPAMIGRATE_CONFIG")]
    pub config: Option<PathBuf>,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Migrate between two Supabase projects
    Migrate(MigrateArgs),

    /// Backup a Supabase project
    Backup(BackupArgs),

    /// Restore from a backup
    Restore(RestoreArgs),

    /// Storage-only operations
    Storage(StorageArgs),

    /// Manage configuration
    Config(ConfigArgs),
}

#[derive(Parser)]
pub struct MigrateArgs {
    /// Source project reference or alias
    #[arg(long, env = "SUPAMIGRATE_SOURCE")]
    pub from: String,

    /// Target project reference or alias
    #[arg(long, env = "SUPAMIGRATE_TARGET")]
    pub to: String,

    /// Include storage objects
    #[arg(long, default_value = "false")]
    pub include_storage: bool,

    /// Include edge functions
    #[arg(long, default_value = "false")]
    pub include_functions: bool,

    /// Schema only (no data)
    #[arg(long, default_value = "false")]
    pub schema_only: bool,

    /// Data only (no schema)
    #[arg(long, default_value = "false")]
    pub data_only: bool,

    /// Exclude specific tables (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub exclude_tables: Option<Vec<String>>,

    /// Exclude specific schemas (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub exclude_schemas: Option<Vec<String>>,

    /// Dry run - show what would be done
    #[arg(long, default_value = "false")]
    pub dry_run: bool,

    /// Skip confirmation prompt
    #[arg(short = 'y', long, default_value = "false")]
    pub yes: bool,
}

#[derive(Parser)]
pub struct BackupArgs {
    /// Project reference or alias to backup
    #[arg(long, env = "SUPAMIGRATE_PROJECT")]
    pub project: String,

    /// Output directory for backup files
    #[arg(short, long, default_value = "./backup")]
    pub output: PathBuf,

    /// Include storage objects in backup
    #[arg(long, default_value = "false")]
    pub include_storage: bool,

    /// Exclude edge functions from backup (functions included by default)
    #[arg(long, default_value = "false")]
    pub no_functions: bool,

    /// Schema only (no data)
    #[arg(long, default_value = "false")]
    pub schema_only: bool,

    /// Compress output with gzip
    #[arg(long, default_value = "true")]
    pub compress: bool,
}

#[derive(Parser)]
pub struct RestoreArgs {
    /// Backup directory or file to restore from
    #[arg(long)]
    pub from: PathBuf,

    /// Target project reference or alias
    #[arg(long, env = "SUPAMIGRATE_TARGET")]
    pub to: String,

    /// Include storage objects
    #[arg(long, default_value = "false")]
    pub include_storage: bool,

    /// Include edge functions
    #[arg(long, default_value = "false")]
    pub include_functions: bool,

    /// Skip confirmation prompt
    #[arg(short = 'y', long, default_value = "false")]
    pub yes: bool,
}

#[derive(Parser)]
pub struct StorageArgs {
    #[command(subcommand)]
    pub command: StorageCommands,
}

#[derive(Subcommand)]
pub enum StorageCommands {
    /// List buckets in a project
    List {
        /// Project reference or alias
        #[arg(long)]
        project: String,
    },

    /// Sync storage between projects
    Sync {
        /// Source project
        #[arg(long)]
        from: String,

        /// Target project
        #[arg(long)]
        to: String,

        /// Specific bucket to sync (all if not specified)
        #[arg(long)]
        bucket: Option<String>,

        /// Number of parallel transfers
        #[arg(long, default_value = "4")]
        parallel: usize,
    },

    /// Download storage to local directory
    Download {
        /// Project reference or alias
        #[arg(long)]
        project: String,

        /// Output directory
        #[arg(short, long, default_value = "./storage-backup")]
        output: PathBuf,

        /// Specific bucket (all if not specified)
        #[arg(long)]
        bucket: Option<String>,
    },

    /// Upload local directory to storage
    Upload {
        /// Source directory
        #[arg(long)]
        from: PathBuf,

        /// Target project
        #[arg(long)]
        to: String,

        /// Target bucket
        #[arg(long)]
        bucket: String,
    },
}

#[derive(Parser)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommands,
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Initialize a new config file
    Init {
        /// Output path
        #[arg(short, long, default_value = "./supamigrate.toml")]
        output: PathBuf,
    },

    /// Add a project to config
    Add {
        /// Project alias
        #[arg(long)]
        alias: String,

        /// Project reference (e.g., abcdefghijklmnop)
        #[arg(long)]
        project_ref: String,

        /// Database password
        #[arg(long)]
        db_password: String,

        /// Service role key (for storage operations)
        #[arg(long)]
        service_key: Option<String>,
    },

    /// List configured projects
    List,

    /// Show current config
    Show,
}
