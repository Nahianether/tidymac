use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "tidymac",
    about = "A macOS cleanup tool — find and remove junk files",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Scan for junk files (dry-run, no deletion)
    Scan {
        /// Only scan a specific category
        #[arg(long)]
        category: Option<String>,

        /// Minimum file size for large-file finder (e.g. "100MB", "1GB")
        #[arg(long, default_value = "100MB")]
        min_size: String,

        /// Root path for .DS_Store scan and large file finder
        #[arg(long)]
        path: Option<String>,
    },

    /// Clean junk files (requires --confirm to actually delete)
    Clean {
        /// Actually delete files. Without this flag, behaves like scan.
        #[arg(long)]
        confirm: bool,

        /// Only clean a specific category
        #[arg(long)]
        category: Option<String>,

        /// Minimum file size for large-file finder (e.g. "100MB", "1GB")
        #[arg(long, default_value = "100MB")]
        min_size: String,

        /// Root path for .DS_Store scan and large file finder
        #[arg(long)]
        path: Option<String>,
    },
}
