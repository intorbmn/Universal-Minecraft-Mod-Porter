# ADR 0015 — CLI output and configuration contract

**Status:** Accepted  
**Date:** 2026-11-20

## Context

Phase 12 exposes UMMP to scripts and CI. Existing commands have inconsistent JSON support,
partly command-local exit behavior, and no shared CLI configuration contract.

## Decision

- Keep the CLI in the existing `cli` crate; do not add a second executable.
- Define stable process statuses: 0 success, 1 general error, 2 safety refusal,
  3 completed with user work remaining, 4 build failure, 5 invalid CLI input.
- Version machine-readable output with a top-level `schema: 1` field and include the
  command name. Logs and diagnostics belong on stderr; JSON belongs on stdout.
- Use `ummp.toml`, loaded from the current directory before the per-user config directory.
- Resolve values in this order: CLI flag > `UMMP_*` environment > config file > built-in.
- Generate shell completions from the clap command tree with `clap_complete`.
- Keep configuration parsing in the existing `core::config::load_any`; add TOML support
  there rather than introducing another parser.

## Consequences

- Scripts can rely on stable exit-code meanings and detect the JSON schema version.
- CLI options remain the highest-priority source of configuration.
- Completion scripts stay synchronized with the actual clap interface.
- The current implementation is incremental: exit codes, TOML configuration, completions,
  and versioned JSON for analyze/scan are in place; global JSON and full command coverage
  remain Phase 12.A acceptance work.

## Alternatives rejected

- A separate CLI configuration dependency: the workspace already has TOML support.
- A hand-maintained completion script: it would drift from clap.
- Async code in `core`: Phase 12 keeps async infrastructure outside the core library.
