# Universal Minecraft Mod Porter (UMMP)

A tool to analyze, plan, and migrate Minecraft mods across versions, mod loaders, and platforms.

## Status
Phases 0-9 complete; Phase 10 (modpacks) is next; rough designs for 10-14 in [docs/drafts](docs/drafts/README.md). Deferred work: [BACKLOG.md](docs/BACKLOG.md). See
[PROJECT_STATUS.md](docs/PROJECT_STATUS.md) and [KNOWN_LIMITATIONS.md](docs/KNOWN_LIMITATIONS.md).

## Documentation
| Doc | Purpose |
|---|---|
| [PRD.md](docs/PRD.md) | Product requirements |
| [SAD.md](docs/SAD.md) | Software architecture |
| [ROADMAP.md](docs/ROADMAP.md) | Phase roadmap |
| [MODULES.md](docs/MODULES.md) | Module catalog |
| [DEPENDENCY_GRAPH.md](docs/DEPENDENCY_GRAPH.md) | Module dependencies |
| [TECH_STACK.md](docs/TECH_STACK.md) | Technology decisions |
| [PLUGIN_SDK.md](docs/PLUGIN_SDK.md) | Plugin architecture |
| [AI_ARCHITECTURE.md](docs/AI_ARCHITECTURE.md) | AI agent architecture |
| [MIGRATION_PIPELINE.md](docs/MIGRATION_PIPELINE.md) | Migration workflow |
| [DATABASE_SCHEMA.md](docs/DATABASE_SCHEMA.md) | Data model |
| [SECURITY.md](docs/SECURITY.md) | Security design |
| [TESTING_STRATEGY.md](docs/TESTING_STRATEGY.md) | Testing strategy |
| [RISK_REGISTER.md](docs/RISK_REGISTER.md) | Risk register |
| [PROJECT_STATUS.md](docs/PROJECT_STATUS.md) | Progress tracking |
| [KNOWN_LIMITATIONS.md](docs/KNOWN_LIMITATIONS.md) | Known limitations |
| [decisions/](docs/decisions/README.md) | Architecture Decision Records |
| [CLAUDE.md](CLAUDE.md) | Workflow rules and skills per phase for AI-assisted work |

## Quick start
```sh
cargo build --release

# Detect loader, Minecraft/Java versions, dependencies, build system
ummp analyze path/to/mod            # add --json for machine-readable output

# Preview a migration (nothing is written)
ummp migrate path/to/mod --from 1.16.5 --to 1.17.1 --loader forge --mappings mappings.json

# Write it (a checkpoint is taken first), then undo if needed
ummp migrate path/to/mod --from 1.16.5 --to 1.17.1 --loader forge --mappings mappings.json --apply --report report.md
ummp rollback path/to/mod

# JAR-only mod: remap class names into a new JAR (input is not modified)
ummp remap-jar mod.jar mod-1.17.jar --from 1.16.5 --to 1.17.1 --loader forge --mappings mappings.json

# AI assistance (needs ANTHROPIC_API_KEY; proposals are reviewed, approved, then applied with a checkpoint)
ummp ai-assist path/to/mod --from 1.16.5 --to 1.17.1 --loader forge --mappings mappings.json
ummp ai-debug path/to/mod --log build.log --from 1.16.5 --to 1.17.1 --loader forge

# Malware-risk scan (exit code 2 on High risk); runs automatically before every build
ummp scan path/to/mod-or.jar
# Build + tests + dedicated-server start on GitHub Actions (needs `gh auth login` and a build repo)
ummp build path/to/mod --repo you/ummp-build-sandbox
# ...or on this machine, in a new folder under D:\Claude\ummp-builds (Windows) or ~/ummp-builds
ummp build path/to/mod --local

# Port a mod: source folder or .zip in, ported source out (the original is not touched).
# Output goes to D:\Claude\ummp-ports\<mod>-<mc>-<loader>-<time> on Windows, ~/ummp-ports elsewhere.
ummp port path/to/mod-src --to 1.21.1 --loader neoforge --zip
ummp adapters    # technologies UMMP detects or migrates (see docs/ADAPTER_SDK.md)
# local model: --provider openai --base-url http://localhost:11434/v1 ; settings: --ai-config ai.json
```
`--mappings` takes a JSON list of `{version, loader, from_api, to_api, kind, source, confidence}`
(the knowledge-base export format); `--kb` takes a knowledge-base SQLite file instead.
Checkpoints go to `<project>.ummp-snapshots/` next to the project unless `--snapshots` is set.

See [ROADMAP.md](docs/ROADMAP.md) for the development plan.