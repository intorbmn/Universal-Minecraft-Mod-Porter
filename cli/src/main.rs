use clap::{ArgGroup, Parser, Subcommand};
use std::io::{BufRead, IsTerminal, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use ummp_ai_providers::{AnthropicProvider, OpenAiCompatibleProvider};
use ummp_core::ai::orchestrator::{AiSessionReport, ApplyTarget, Orchestrator};
use ummp_core::ai::{AiConfig, AiProvider, ApprovalGate, ApproveReviewed, AuditLog, DenyAll};
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
    /// AI review of a migration: analysis, proposed fixes, review, then your approval.
    #[command(group(ArgGroup::new("knowledge").required(true).args(["kb", "mappings"])))]
    AiAssist {
        /// Project root path.
        path: PathBuf,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        loader: String,
        #[arg(long)]
        kb: Option<PathBuf>,
        #[arg(long)]
        mappings: Option<PathBuf>,
        #[command(flatten)]
        ai: AiArgs,
    },
    /// AI fix proposals for a build or crash log, reviewed, then your approval.
    AiDebug {
        /// Project root path.
        path: PathBuf,
        /// Build or crash log file.
        #[arg(long)]
        log: PathBuf,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        loader: String,
        #[command(flatten)]
        ai: AiArgs,
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

#[derive(clap::Args)]
struct AiArgs {
    /// `anthropic` (ANTHROPIC_API_KEY) or `openai` (OpenAI-compatible, e.g. Ollama).
    #[arg(long, default_value = "anthropic")]
    provider: String,
    /// Base URL for `--provider openai`, e.g. http://localhost:11434/v1.
    #[arg(long)]
    base_url: Option<String>,
    /// YAML/JSON AI config: per-agent models, budget, retries, prices.
    #[arg(long)]
    ai_config: Option<PathBuf>,
    /// Apply every change the review agent approved without asking.
    /// Changes the reviewer rejected are never applied by this flag.
    #[arg(long)]
    approve_reviewed: bool,
    /// Checkpoint and audit directory.
    #[arg(long)]
    snapshots: Option<PathBuf>,
}

/// Asks on the terminal for each proposed change.
struct PromptGate;

impl ApprovalGate for PromptGate {
    fn approve(&mut self, path: &Path, diff: &str, reviewer_ok: bool, concerns: &[String]) -> bool {
        println!("\n{diff}");
        println!(
            "Review agent: {}",
            if reviewer_ok {
                "approves"
            } else {
                "does NOT approve"
            }
        );
        for c in concerns {
            println!("  - {c}");
        }
        print!("Apply this change to {}? [y/N] ", path.display());
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line).is_ok()
            && matches!(line.trim(), "y" | "Y" | "yes")
    }
}

fn provider(args: &AiArgs) -> Result<Box<dyn AiProvider>, Box<dyn std::error::Error>> {
    Ok(match args.provider.as_str() {
        "anthropic" => Box::new(AnthropicProvider::from_env()?),
        "openai" => {
            let url = args
                .base_url
                .as_deref()
                .ok_or("--provider openai needs --base-url")?;
            Box::new(OpenAiCompatibleProvider::new(url))
        }
        other => return Err(format!("unknown provider '{other}'").into()),
    })
}

fn gate(args: &AiArgs) -> Box<dyn ApprovalGate> {
    if args.approve_reviewed {
        Box::new(ApproveReviewed)
    } else if std::io::stdin().is_terminal() {
        Box::new(PromptGate)
    } else {
        eprintln!("No terminal to ask for approval: proposals are shown, nothing is applied.");
        Box::new(DenyAll)
    }
}

fn print_ai_report(r: &AiSessionReport, cfg: &AiConfig) {
    if let Some(a) = &r.analysis {
        println!("\n## Analysis\n{}", a.summary);
        for risk in &a.risks {
            println!("- [{}] {}: {}", risk.severity, risk.area, risk.detail);
        }
    }
    for n in &r.notes {
        println!("note: {n}");
    }
    for x in &r.rejected {
        println!("rejected {}: {}", x.path, x.reason);
    }
    for o in &r.omitted_context {
        println!("not sent to the model (too large): {o}");
    }
    println!(
        "\nProposed: {}, applied: {}",
        r.proposed.len(),
        r.applied.len()
    );
    if let Some(id) = r.checkpoint_id {
        println!("Checkpoint: {id} (ummp rollback restores it)");
    }
    print!("Tokens: {}", r.cost.total_tokens());
    match r.cost.cost_usd(&cfg.prices) {
        Some(usd) => println!(" (~${usd:.4})"),
        None => println!(" (set prices in --ai-config to see cost)"),
    }
}

fn ai_config(args: &AiArgs) -> Result<AiConfig, Box<dyn std::error::Error>> {
    Ok(match &args.ai_config {
        Some(p) => ummp_core::config::load_any(p)?,
        None => AiConfig::default(),
    })
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

        Commands::AiAssist {
            path,
            from,
            to,
            loader,
            kb,
            mappings,
            ai,
        } => {
            let project = pm.open(&path)?;
            let kb = open_kb(kb, mappings)?;
            let request = MigrationRequest {
                source_minecraft: from.clone(),
                target_minecraft: to.clone(),
                target_loader: loader.to_lowercase(),
                min_confidence: 0.8,
            };
            let plan = MigrationPlanner::plan(&kb, &request)?;
            let snap_dir = ai
                .snapshots
                .clone()
                .unwrap_or_else(|| default_snapshots(&project.root));
            let mut store = load_store(&snap_dir)?;
            let engine = MigrationEngine::default();
            // Rule-based dry run: its report and warnings are the agents' input.
            let rules = engine.run(
                &plan,
                &project,
                &mut store,
                &RunOptions {
                    dry_run: true,
                    snapshot_root: &snap_dir,
                    session_id: Uuid::new_v4(),
                },
            )?;
            let fs: Arc<dyn FileSystem> = Arc::new(LocalFileSystem);
            let analysis = AnalyzerEngine::default().analyze(&project.root, &fs)?;
            let cfg = ai_config(&ai)?;
            let provider = provider(&ai)?;
            let mut gate = gate(&ai);
            let mut orch = Orchestrator::new(
                provider.as_ref(),
                cfg.clone(),
                AuditLog::new(Some(&snap_dir.join("ai-audit.jsonl"))),
            );
            let report = orch.assist(
                &format!("Minecraft {from} -> {to} on {loader}"),
                &analysis.to_text(),
                &rules.to_markdown(),
                &rules.warnings,
                gate.as_mut(),
                &mut ApplyTarget {
                    project: &project,
                    engine: &engine,
                    store: &mut store,
                    snapshot_root: &snap_dir,
                },
            )?;
            if report.checkpoint_id.is_some() {
                save_store(&snap_dir, &store)?;
            }
            print_ai_report(&report, &cfg);
        }

        Commands::AiDebug {
            path,
            log,
            from,
            to,
            loader,
            ai,
        } => {
            let project = pm.open(&path)?;
            let log_text = std::fs::read_to_string(&log)?;
            let snap_dir = ai
                .snapshots
                .clone()
                .unwrap_or_else(|| default_snapshots(&project.root));
            let mut store = load_store(&snap_dir)?;
            let engine = MigrationEngine::default();
            let cfg = ai_config(&ai)?;
            let provider = provider(&ai)?;
            let mut gate = gate(&ai);
            let mut orch = Orchestrator::new(
                provider.as_ref(),
                cfg.clone(),
                AuditLog::new(Some(&snap_dir.join("ai-audit.jsonl"))),
            );
            let report = orch.debug(
                &format!("Minecraft {from} -> {to} on {loader}"),
                &log_text,
                gate.as_mut(),
                &mut ApplyTarget {
                    project: &project,
                    engine: &engine,
                    store: &mut store,
                    snapshot_root: &snap_dir,
                },
            )?;
            if report.checkpoint_id.is_some() {
                save_store(&snap_dir, &store)?;
            }
            print_ai_report(&report, &cfg);
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
