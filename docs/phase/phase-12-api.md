# Phase 12 — CLI, API & Integration

**Status:** IN PROGRESS  
**Branch:** `phase-12-api` (based on merged Phase 11 `master`)  
**Guide:** [Phase 12 API guide](../guides/phase-12-api.md)  
**Design history:** [Phase 12 draft](../drafts/phase-12-api.md)  
**ADRs:** [0015 — CLI output and configuration contract](../decisions/0015-cli-output-and-config.md) (preserved); [0016 — interim AI key storage](../decisions/0016-interim-ai-key-file-storage.md)\n**Design:** [approved specification](../superpowers/specs/2026-11-20-phase12-api-design.md)\n**Plan:** [implementation plan](../superpowers/plans/2026-11-20-phase12-api.md)

## Working method

The phase guide names Claude Code skills that are not installed in this session. The user
approved continuing manually. Apply their practices by hand: design before implementation,
small TDD changes, explicit security review, and verification before marking a milestone done.
Prefer the smallest correct implementation and do not add dependencies when the workspace
already provides the needed capability.

## Dependency boundary

Phase 11 was merged into `master` by PR #11 on 2026-11-20. Phase 12 therefore starts from
the merged `AppServices` facade. The async runtime, HTTP server and transport adapters must
remain outside `core`.

## Milestone status

| Milestone | Status | Notes |
|---|---|---|
| 12.A CLI polish | IN PROGRESS | Stable exit-code type, clap completions, TOML config loading and precedence helpers; JSON envelope for analyze/scan. Global JSON and complete command coverage/tests remain. |
| 12.B REST API | NOT STARTED | Planned in a separate `server/` workspace member; localhost + bearer token. |
| 12.C MCP server | NOT STARTED | stdio transport; every write must use safe application services and explicit approval. |
| 12.D GitHub Action | NOT STARTED | Composite action; build from source until Phase 13 release binaries exist. |
| 12.E Plugin interface | NOT STARTED | Out-of-process JSON-lines protocol; plugins return edits and never write project files. |

## D2 / P1 design record\n\n- User approved the Phase 12 design specification on 2026-11-20.\n- Added ADR 0016, status Accepted (interim), for file-backed AI credentials.\n- ADR 0015 already existed as Accepted for the CLI contract. It is immutable and remains unchanged; later Phase 12 ADRs are shifted by one (REST 0017, MCP 0018, plugins 0019).\n- Added the 12.0-first implementation plan. Execution is manual without skill installation, as the user explicitly approved.\n- No implementation code has been changed yet.\n\n## 12.A implementation record

- Added `cli/src/exit.rs` with the six documented numeric exit statuses.
- Added `clap_complete` and `ummp completions bash|zsh|fish|powershell`.
- Added `cli/src/config.rs` to load `ummp.toml` from the current directory, then the user
  config directory; environment values override file values and CLI values override config.
- Applied configured output root, sandbox repository and AI provider to existing commands.
- Extended `core::config::load_any` with TOML support (the `toml` dependency already existed).
- Added a versioned `schema: 1` JSON envelope for `analyze --json` and `scan --json`.
- Added typed failure mapping for safety refusal (2), invalid command input (5), and build
  backend/stage failure (4); generic failures remain 1.
- Added CLI integration tests for invalid-argument/help/build-input exit codes, completions,
  TOML flag precedence and the scan JSON envelope. CI now runs on `phase-12-api`.

## Outstanding 12.A acceptance

1. Make `--json` a global flag and provide structured JSON for every command, with logs on stderr.
2. Ensure runtime input errors, safety refusals and build failures map consistently to codes 5, 2 and 4.
3. Add golden tests for stable JSON keys and test config-file defaults overridden by flags.
4. Verify bash, zsh, fish and PowerShell completions in CI.
5. Run build, tests, Clippy and rustfmt on the branch; fix every failure before closing 12.A.

## Decisions and open work

- REST framework default: `axum + tokio`, recorded in its own ADR before implementation.
- REST default bind: `127.0.0.1`; non-loopback requires an explicit acknowledgement flag.
- MCP default transport: stdio.
- Plugin execution: child process with bounded JSON-lines input/output, timeout and path validation.
- Do not merge this phase or begin Phase 13 without the user's review.
