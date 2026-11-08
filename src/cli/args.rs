use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "vibe-commit")]
#[command(about = "Monolithic AI diff chunker & atomic conventional commit planner")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Fallback patch file if run without subcommand
    #[arg(value_name = "PATCH_FILE")]
    pub default_patch: Option<PathBuf>,

    /// Output format for the commit plan
    #[arg(long, short = 'f', value_enum, default_value = "markdown")]
    pub format: OutputFormat,

    /// Maximum number of atomic commits to generate
    #[arg(long, short = 'm')]
    pub max_commits: Option<usize>,

    /// Optional file path to write output to
    #[arg(long, short = 'o')]
    pub output: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate an atomic conventional commit plan from a unified diff
    Plan {
        /// Path to unified diff / patch file (use '-' or omit for stdin)
        #[arg(value_name = "PATCH_FILE")]
        patch_file: Option<PathBuf>,

        /// Output format
        #[arg(long, short = 'f', value_enum, default_value = "markdown")]
        format: OutputFormat,

        /// Maximum number of atomic commits to generate
        #[arg(long, short = 'm')]
        max_commits: Option<usize>,

        /// Optional file path to write plan output
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },
    /// Apply planned atomic commits sequentially (supports --dry-run)
    Apply {
        /// Path to unified diff / patch file (use '-' or omit for stdin)
        #[arg(value_name = "PATCH_FILE")]
        patch_file: Option<PathBuf>,

        /// Dry-run mode: prints commands and validations without executing git changes
        #[arg(long)]
        dry_run: bool,

        /// Maximum number of atomic commits to generate
        #[arg(long, short = 'm')]
        max_commits: Option<usize>,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Markdown,
    Json,
    Shell,
}
