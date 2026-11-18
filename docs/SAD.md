# SAD — Software Architecture Document

## Architecture style
Modular monolith. Layers: Domain → Application → Infrastructure/Presentation. Core engine has no hard dependency on GUI, AI provider, Minecraft loader, or build tool.

## Layer rules
- **Domain**: pure business logic (project, migration session, compatibility). No GUI, DB, AI, or loader imports.
- **Application**: use cases, orchestration, pipeline. Depends on Domain + interfaces only.
- **Infrastructure**: adapters (loaders, mappings, build, AI providers, DB, sandbox). Implement interfaces.
- **Presentation**: desktop GUI, CLI, REST API. Call application services only.

## Core modules
| Module | Responsibility |
|---|---|
| Project Manager | open/import project, metadata, file system abstraction |
| Migration Session | checkpoint, rollback, job persistence, state machine |
| Event Bus | decoupled communication between pipeline steps |
| Analyzer Engine | detect loader/version/Java/deps/build system |
| Compatibility Engine | mapping DB, breaking changes, recipes, knowledge search |
| Migration Engine | plan + transform source/build/resources/mixins |
| Build Engine | Java/Gradle/Maven runner, launcher, log/crash analysis |
| AI Agent System | provider abstraction, agents, context, cost tracking, approval gate |
| Plugin System | registry, loader, lifecycle, capability discovery, sandbox |

## Key patterns
- **Adapter**: wraps a specific loader/build/ai technology behind an interface.
- **Strategy**: interchangeable implementations (e.g. Gradle vs Maven runner).
- **Repository**: abstract storage of mappings/recipes/compatibility data.
- **Plugin Registry**: dynamic capability discovery at runtime.
- **Pipeline**: each step has input/output/state/log/timeout/retry/checkpoint/resume.

## Data flow (migration)
Import → Detect → Analyze → Plan → Transform → Build → Test → Review → Export

## Cross-cutting
- Security: untrusted input (JARs, repos, plugins) is sandboxed; permission model; audit logging.
- Observability: structured logs, metrics, tracing, diagnostic reports, job state visible to user.
- Versioning: project, session, schema, plugin API, mapping, recipe, artifact all versioned. Every migration is reproducible.

## Diagrams
See `docs/architecture/` for Mermaid diagrams.