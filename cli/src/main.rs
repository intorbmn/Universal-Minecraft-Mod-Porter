use clap::{Parser, Subcommand};
use std::path::PathBuf;
use ummp_core::project::{DefaultProjectManager, ProjectManager};

#[derive(Parser)]
#[command(name = "ummp", version, about = "Universal Minecraft Mod Porter")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Open a project and print detected metadata.
    Analyze {
        /// Project root path.
        path: PathBuf,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze { path } => {
            let pm = DefaultProjectManager;
            let project = pm.open(&path)?;
            let meta = pm.metadata(&project)?;
            println!("{}", serde_json::to_string_pretty(&meta)?);
        }
    }

    Ok(())
}
