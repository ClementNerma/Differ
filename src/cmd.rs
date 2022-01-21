use std::path::PathBuf;

use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Source directory
    #[clap(help = "Source directory (files to backup)")]
    pub source_dir: PathBuf,

    /// Backup directory
    #[clap(help = "Backup directory (where to write the backup files)")]
    pub backup_dir: PathBuf,

    /// Differential directory
    #[clap(help = "Differential directory (intermediary backup)")]
    pub diff_dir: PathBuf,
}
