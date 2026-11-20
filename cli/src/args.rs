//! Clap argument definitions for the UMMP command line.

use clap::{ArgGroup, Parser, Subcommand};
use clap_complete::Shell;
use std::path::PathBuf;

#[derive(Subcommand)]
pub(crate) enum PackCmd {
    /// Read a pack (.mrpack, CurseForge .zip, Prism instance or folder) and report. Writes nothing.
    Inspect {
        pack: PathBuf,
        #[arg(long)]
        from_loader: Option<String>,
        #[arg(long)]
        from_mc: Option<String>,
    },
    /// Migrate a pack to another Minecraft version and/or loader.
    Migrate {
        pack: PathBuf,
        /// Target Minecraft version.
        #[arg(long)]
        to: String,
        /// Target loader: forge, fabric, neoforge or quilt.
        #[arg(long)]
        loader: String,
        #[arg(long)]
        from_loader: Option<String>,
        #[arg(long)]
        from_mc: Option<String>,
        /// Ask Modrinth (read-only) for versions of each mod that fit the target.
        #[arg(long)]
        online: bool,
        /// Folder with `<mod-id>/` or `<mod-id>.zip` sources for mods you want ported.
        #[arg(long)]
        sources: Option<PathBuf>,
        /// client, server or both.
        #[arg(long, default_value = "both")]
        side: String,
        /// Output folder (default: <packs root>/<pack>-<mc>-<loader>-<time>).
        #[arg(long)]
        output: Option<PathBuf>,
        /// Continue even when a JAR is rated High risk (you reviewed it; the report keeps the rating).
        #[arg(long)]
        allow_risk: bool,
        #[arg(long)]
        loader_version: Option<String>,
        /// Comma list of mrpack,curseforge,prism,folder.
        #[arg(long, value_delimiter = ',', default_value = "mrpack,folder")]
        format: Vec<String>,
        /// Knowledge base for porting mods from `--sources`.
        #[arg(long)]
        kb: Option<PathBuf>,
        #[arg(long)]
        mappings: Option<PathBuf>,
        #[arg(long, default_value_t = 0.8)]
        min_confidence: f64,
    },
    /// Convert a pack to another format without changing versions.
    Export {
        pack: PathBuf,
        #[arg(long)]
        to_format: String,
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

#[derive(Parser)]
#[command(name = "ummp", version, about = "Universal Minecraft Mod Porter")]
pub(crate) struct Cli {
    /// Emit a versioned JSON envelope; human-readable output moves to stderr.
    #[arg(long, global = true)]
    pub(crate) json: bool,
    /// Explicit TOML configuration file.
    #[arg(long, global = true, value_name = "PATH")]
    pub(crate) config: Option<PathBuf>,
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Generate shell completions.
    Completions { shell: Shell },
    /// Detect loader, Minecraft/Java versions, dependencies and build system.
    Analyze {
        /// Project root path (source tree or a folder of JARs).
        path: PathBuf,
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
        /// Loader the mod uses now (default: detected). A different `--loader` converts it.
        #[arg(long)]
        from_loader: Option<String>,
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
    /// Static malware-risk scan of a mod project or JAR. Exit code 2 when High risk.
    Scan {
        /// Project folder or JAR file.
        path: PathBuf,
    },
    /// Build, test and start a dedicated server; on GitHub Actions unless `--local`.
    Build {
        /// Project root path (source tree with Gradle build files).
        path: PathBuf,
        #[command(flatten)]
        opts: BuildArgs,
    },
    /// Port a mod's source: input folder or .zip in, ported source out (input untouched).
    Port {
        /// Source folder or .zip of the mod.
        input: PathBuf,
        /// Target Minecraft version.
        #[arg(long)]
        to: String,
        /// Target loader (fabric, quilt, forge, neoforge); default: the mod's loader.
        #[arg(long)]
        loader: Option<String>,
        /// Source Minecraft version; default: detected from the mod descriptor.
        #[arg(long)]
        from: Option<String>,
        /// Output folder (default: D:\Claude\ummp-ports\<mod>-<mc>-<loader>-<time> on
        /// Windows, ~/ummp-ports/... elsewhere). Must be empty or missing.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Also write <output>.zip.
        #[arg(long)]
        zip: bool,
        /// Knowledge base SQLite file with API mappings.
        #[arg(long, conflicts_with = "mappings")]
        kb: Option<PathBuf>,
        /// Mappings JSON (knowledge-base export format).
        #[arg(long)]
        mappings: Option<PathBuf>,
        #[arg(long, default_value_t = 0.8)]
        min_confidence: f64,
        /// Build, test and start a server with the ported source (see `ummp build`).
        #[arg(long)]
        build: bool,
        #[command(flatten)]
        build_opts: BuildArgs,
    },
    /// List technology adapters and how far each one goes.
    Adapters,
    /// Modpacks: inspect, migrate to another Minecraft version/loader, convert between formats.
    /// Exit codes: 0 done, 2 refused (High-risk JAR, use --allow-risk), 3 done but some mods
    /// are left for you (see UMMP_PACK_REPORT.md).
    Pack {
        #[command(subcommand)]
        cmd: PackCmd,
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
pub(crate) struct BuildArgs {
    /// Build on this machine instead of GitHub.
    #[arg(long)]
    pub(crate) local: bool,
    /// GitHub build repository `owner/name` (default: $UMMP_BUILD_REPO or
    /// intorbmn/ummp-build-sandbox).
    #[arg(long)]
    pub(crate) repo: Option<String>,
    /// Folder for build copies (default: D:\Claude\ummp-builds on Windows, ~/ummp-builds).
    #[arg(long)]
    pub(crate) workdir: Option<PathBuf>,
    /// Comma-separated stages: build,test,server.
    #[arg(long, value_delimiter = ',', default_value = "build,test,server")]
    pub(crate) stages: Vec<String>,
    /// Limit per stage, in seconds.
    #[arg(long, default_value_t = 1800)]
    pub(crate) timeout: u64,
    /// Java major version (default: detected from the mod).
    #[arg(long)]
    pub(crate) java: Option<u32>,
    /// Build a High-risk mod on this machine anyway.
    #[arg(long)]
    pub(crate) i_accept_risk: bool,
    /// Build a Medium-risk mod on this machine without asking.
    #[arg(long)]
    pub(crate) yes: bool,
    /// Also write the Markdown report here.
    #[arg(long)]
    pub(crate) report: Option<PathBuf>,
}

#[derive(clap::Args)]
pub(crate) struct AiArgs {
    /// `anthropic` (ANTHROPIC_API_KEY) or `openai` (OpenAI-compatible, e.g. Ollama).
    #[arg(long)]
    pub(crate) provider: Option<String>,
    /// Base URL for `--provider openai`, e.g. http://localhost:11434/v1.
    #[arg(long)]
    pub(crate) base_url: Option<String>,
    /// YAML/JSON AI config: per-agent models, budget, retries, prices.
    #[arg(long)]
    pub(crate) ai_config: Option<PathBuf>,
    /// Apply every change the review agent approved without asking.
    /// Changes the reviewer rejected are never applied by this flag.
    #[arg(long)]
    pub(crate) approve_reviewed: bool,
    /// Checkpoint and audit directory.
    #[arg(long)]
    pub(crate) snapshots: Option<PathBuf>,
}
