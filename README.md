# Universal Minecraft Mod Porter (UMMP)

A tool to analyze, plan, and migrate Minecraft mods across versions, mod loaders, and platforms.

## Status
Phases 0-5 complete; Phase 6 (migration engine) in progress. See
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
```
`--mappings` takes a JSON list of `{version, loader, from_api, to_api, kind, source, confidence}`
(the knowledge-base export format); `--kb` takes a knowledge-base SQLite file instead.
Checkpoints go to `<project>.ummp-snapshots/` next to the project unless `--snapshots` is set.

See [ROADMAP.md](docs/ROADMAP.md) for the development plan.