use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Source directory
    #[clap(help = "Source directory")]
    pub source_dir: String,

    /// Destination directory
    #[clap(help = "Destination directory (to synchronize with the source directory)")]
    pub dest_dir: String,
}
