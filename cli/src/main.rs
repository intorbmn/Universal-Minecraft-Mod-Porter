use clap::{ArgGroup, Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use ummp_core::analyzer::AnalyzerEngine;
use ummp_core::bytecode::{remap_jar, ClassMap};
use ummp_core::checkpoint::CheckpointStore;
use ummp_core::filesystem::{FileSystem, LocalFileSystem};
use ummp_core::knowledge::KnowledgeBase;
use ummp_core::migration::{MigrationEngine, MigrationPlanner, MigrationRequest, RunOptions};
use ummp_core::project::{DefaultProjectManager, ProjectManager};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "ummp", version, about = "Universal Minecraft Mod Porter")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Detect loader, Minecraft/Java versions, dependencies and build system.
    Analyze {
        /// Project root path (source tree or a folder of JARs).
        path: PathBuf,
        /// Print the machine-readable JSON report instead of text.
        #[arg(long)]
        json: bool,
    },
    /// Plan and run a source migration. Dry run unless `--apply` is given.
    #[command(group(ArgGroup::new("knowledge").required(true).args(["kb", "mappings"])))]
    Migrate {
        /// Project root path.
        path: PathBuf,
        /// Source Minecraft version, e.g. 1.16.5.
        #[arg(long)]
        from: String,
        /// Target Minecraft version, e.g. 1.17.1.
        #[arg(long)]
        to: String,
        /// Target loader: forge, fabric, neoforge, quilt.
        #[arg(long)]
        loader: String,
        /// Knowledge-base SQLite file (read only).
        #[arg(long)]
        kb: Option<PathBuf>,
        /// JSON file with a list of mappings (same format as KB export).
        #[arg(long)]
        mappings: Option<PathBuf>,
        /// Mappings below this confidence are reported, not applied.
        #[arg(long, default_value_t = 0.8)]
        min_confidence: f64,
        /// Write the changes (a checkpoint is taken first).
        #[arg(long)]
        apply: bool,
        /// Checkpoint directory; defaults to `<project>.ummp-snapshots` next to the project.
        #[arg(long)]
        snapshots: Option<PathBuf>,
        /// Also write the report to this file (`.json` or Markdown).
        #[arg(long)]
        report: Option<PathBuf>,
    },
    /// Remap class names inside a compiled mod JAR into a new JAR (input untouched).
    #[command(group(ArgGroup::new("knowledge").required(true).args(["kb", "mappings"])))]
    RemapJar {
        /// Input mod JAR.
        input: PathBuf,
        /// Output JAR; must not exist.
        output: PathBuf,
        /// Source Minecraft version.
        #[arg(long)]
        from: String,
        /// Target Minecraft version.
        #[arg(long)]
        to: String,
        /// Target loader.
        #[arg(long)]
        loader: String,
        /// Knowledge-base SQLite file (read only).
        #[arg(long)]
        kb: Option<PathBuf>,
        /// JSON file with a list of mappings.
        #[arg(long)]
        mappings: Option<PathBuf>,
        /// Mappings below this confidence are not applied.
        #[arg(long, default_value_t = 0.8)]
        min_confidence: f64,
    },
    /// Restore the project to a checkpoint taken by `migrate --apply`.
    Rollback {
        /// Project root path.
        path: PathBuf,
        /// Checkpoint directory used by `migrate`.
        #[arg(long)]
        snapshots: Option<PathBuf>,
        /// Checkpoint id; defaults to the latest one.
        #[arg(long)]
        checkpoint: Option<Uuid>,
    },
}

const STORE_FILE: &str = "checkpoints.json";

fn default_snapshots(project_root: &Path) -> PathBuf {
    let name = project_root
        .file_name()
        .map_or_else(|| "project".into(), |n| n.to_string_lossy().into_owned());
    project_root.with_file_name(format!("{name}.ummp-snapshots"))
}

fn load_store(dir: &Path) -> Result<CheckpointStore, Box<dyn std::error::Error>> {
    let file = dir.join(STORE_FILE);
    if file.exists() {
        Ok(serde_json::from_slice(&std::fs::read(file)?)?)
    } else {
        Ok(CheckpointStore::new())
    }
}

fn save_store(dir: &Path, store: &CheckpointStore) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join(STORE_FILE), serde_json::to_vec_pretty(store)?)?;
    Ok(())
}

fn open_kb(
    kb: Option<PathBuf>,
    mappings: Option<PathBuf>,
) -> Result<KnowledgeBase, Box<dyn std::error::Error>> {
    Ok(match (kb, mappings) {
        (Some(db), _) => KnowledgeBase::open(&db)?,
        (None, Some(file)) => {
            let mem = KnowledgeBase::open(Path::new(":memory:"))?;
            mem.import_mappings_from_file(&file)?;
            mem
        }
        (None, None) => unreachable!("clap requires --kb or --mappings"),
    })
}

fn main() {
    tracing_subscriber::fmt::init();
    if let Err(e) = run(Cli::parse()) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let pm = DefaultProjectManager;

    match cli.command {
        Commands::Analyze { path, json } => {
            let project = pm.open(&path)?;
            let fs: Arc<dyn FileSystem> = Arc::new(LocalFileSystem);
            let report = AnalyzerEngine::default().analyze(&project.root, &fs)?;
            if json {
                println!("{}", report.to_json()?);
            } else {
                print!("{}", report.to_text());
            }
        }

        Commands::Migrate {
            path,
            from,
            to,
            loader,
            kb,
            mappings,
            min_confidence,
            apply,
            snapshots,
            report,
        } => {
            let project = pm.open(&path)?;
            let kb = open_kb(kb, mappings)?;
            let request = MigrationRequest {
                source_minecraft: from,
                target_minecraft: to,
                target_loader: loader.to_lowercase(),
                min_confidence,
            };
            let plan = MigrationPlanner::plan(&kb, &request)?;
            let snap_dir = snapshots.unwrap_or_else(|| default_snapshots(&project.root));
            let mut store = load_store(&snap_dir)?;

            let result = MigrationEngine::default().run(
                &plan,
                &project,
                &mut store,
                &RunOptions {
                    dry_run: !apply,
                    snapshot_root: &snap_dir,
                    session_id: Uuid::new_v4(),
                },
            )?;
            if result.checkpoint_id.is_some() {
                save_store(&snap_dir, &store)?;
            }

            let md = result.to_markdown();
            print!("{md}");
            if let Some(out) = report {
                let is_json = out.extension().is_some_and(|e| e == "json");
                let body = if is_json { result.to_json()? } else { md };
                std::fs::write(&out, body)?;
            }
            if !apply && !result.changes.is_empty() {
                eprintln!("\nDry run: nothing was written. Re-run with --apply to write.");
            }
        }

        Commands::RemapJar {
            input,
            output,
            from,
            to,
            loader,
            kb,
            mappings,
            min_confidence,
        } => {
            let kb = open_kb(kb, mappings)?;
            let request = MigrationRequest {
                source_minecraft: from,
                target_minecraft: to,
                target_loader: loader.to_lowercase(),
                min_confidence,
            };
            let plan = MigrationPlanner::plan(&kb, &request)?;
            let stages: Vec<ClassMap> = plan
                .stages
                .iter()
                .map(|s| ClassMap::from_rules(&s.rules))
                .filter(|m| !m.is_empty())
                .collect();
            if stages.is_empty() {
                return Err("no qualified class mappings for this request".into());
            }
            let stats = remap_jar(&input, &output, &stages)?;
            println!(
                "Wrote {}: {} classes remapped, {} class files moved, {} signature files dropped",
                output.display(),
                stats.classes_changed,
                stats.entries_moved,
                stats.signatures_dropped
            );
        }

        Commands::Rollback {
            path,
            snapshots,
            checkpoint,
        } => {
            let project = pm.open(&path)?;
            let snap_dir = snapshots.unwrap_or_else(|| default_snapshots(&project.root));
            let store = load_store(&snap_dir)?;
            let id = match checkpoint {
                Some(id) => id,
                None => {
                    store
                        .latest()
                        .ok_or("no checkpoints found in the snapshot directory")?
                        .id
                }
            };
            store.restore(id, &project.root)?;
            println!("Restored {} to checkpoint {id}", project.root.display());
        }
    }

    Ok(())
}
