# Phase 12 — CLI, API & Integration

**Status:** IN PROGRESS  
**Branch:** `phase-12-api` (based on merged Phase 11 `master`)  
**Guide:** [Phase 12 API guide](../guides/phase-12-api.md)  
**Design history:** [Phase 12 draft](../drafts/phase-12-api.md)  
**ADRs:** [0015 — interim AI key storage](../decisions/0015-ai-key-file-storage.md); REST 0016, MCP 0017 and plugins 0018 are planned  
**Design:** [approved specification](../superpowers/specs/2026-11-20-phase12-api-design.md)  
**Plan:** [implementation plan](../superpowers/plans/2026-11-20-phase12-api.md)

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
| 12.0 AI credentials | IN PROGRESS | Core file store, desktop commands/UI and CLI key commands are implemented. CI passes; manual acceptance and security review remain. |
| 12.A CLI polish | IN PROGRESS | Global `--json`, config and exit-code foundations exist; analyze/scan use `AppServices`. Remaining command routing, JSON coverage and acceptance tests remain. |
| 12.B REST API | NOT STARTED | Planned in a separate `server/` workspace member; localhost + bearer token. |
| 12.C MCP server | NOT STARTED | stdio transport; every write must use safe application services and explicit approval. |
| 12.D GitHub Action | NOT STARTED | Composite action; build from source until Phase 13 release binaries exist. |
| 12.E Plugin interface | NOT STARTED | Out-of-process JSON-lines protocol; plugins return edits and never write project files. |

## D2 / P1 design record

- User approved the Phase 12 design specification on 2026-11-20.
- Added ADR 0015, status Accepted (interim), for file-backed AI credentials.
- Reconciled ADR numbering to the Phase 12 guide: ADR 0015 is interim AI credential storage; the CLI contract remains documented in `docs/CLI.md`; later ADRs are REST 0016, MCP 0017 and plugins 0018.
- Added the 12.0-first implementation plan. Execution is manual without skill installation, as the user explicitly approved.
- At the design approval checkpoint, implementation code had not yet been changed.

## 12.A implementation record

- Added `cli/src/exit.rs` with the six documented numeric exit statuses.
- Added `clap_complete` and `ummp completions bash|zsh|fish|powershell`.
- Added `cli/src/config.rs` with strict TOML fields: `output_root`, `kb`,
  `sandbox_repo`, `ai_provider`, and `openai_base_url`. It supports explicit
  `--config PATH`, current-directory config, then the shared UMMP data directory.
- Environment values override TOML; CLI values override both. Existing
  `UMMP_OUTPUT_DIR` and `UMMP_PROVIDER` remain accepted for compatibility.
- Rejects unknown TOML fields, including key-like fields, and maps missing/invalid
  explicit config files to exit code 5 without echoing secret values.
- Applies configured output root, KB path, sandbox repository, AI provider and OpenAI
  base URL to the existing command paths.
- Extended `core::config::load_any` with TOML support (the `toml` dependency already existed).
- Added a versioned `schema: 1` JSON envelope for `analyze --json` and `scan --json`.
- Added typed failure mapping for safety refusal (2), invalid command input (5), and build
  backend/stage failure (4); generic failures remain 1.
- Added config unit tests for field loading, explicit/local/shared lookup precedence,
  environment overrides, blank environment values, and secret-field rejection.
- Added CLI integration tests for explicit config loading, missing config, and secret-field
  rejection without value leakage, while retaining existing exit/completion/JSON tests.
- Updated `docs/CLI.md` to match the shared data directory, supported fields, environment
  variables and `--config` behavior.
- CI run [#44852188873](https://github.com/intorbmn/Universal-Minecraft-Mod-Porter/actions/runs/44852188873) on `b5e66e8`: both `build-test-lint` and `desktop` passed, including Rust build/tests/Clippy/rustfmt and desktop checks/Vitest/Playwright. This verifies the branch state through that commit; it does not by itself close the remaining manual acceptance items.

## Outstanding 12.A acceptance

1. Provide structured JSON result views for every command, with logs on stderr.
2. Ensure runtime input errors, safety refusals and build failures map consistently to codes 5, 2 and 4.
3. Extend golden coverage to command result views and test config-file defaults overridden by flags.
4. Verify bash, zsh, fish and PowerShell completions in CI.
5. Keep build, tests, Clippy and rustfmt green after each change; complete manual acceptance before closing 12.A.

## Decisions and open work

- REST framework default: `axum + tokio`, recorded in its own ADR before implementation.
- REST default bind: `127.0.0.1`; non-loopback requires an explicit acknowledgement flag.
- MCP default transport: stdio.
- Plugin execution: child process with bounded JSON-lines input/output, timeout and path validation.
- Do not merge this phase or begin Phase 13 without the user's review.

## 12.A follow-up — AppServices routing

- CI run [#44852188873](https://github.com/intorbmn/Universal-Minecraft-Mod-Porter/actions/runs/44852188873) on `b5e66e8`: both jobs completed successfully. Root and desktop build, tests, Clippy and formatting checks passed; desktop web checks, Vitest and Playwright passed.
- Fixed the desktop OpenAI provider call to borrow the configured base URL; commit `e8a83ee`.
- Routed both human-readable and JSON `analyze` and `scan` commands through `AppServices`, removing their direct CLI calls to the analyzer/scanner. Human output now renders the corresponding serializable app views.
- Commit: `c394737f` (`refactor(cli): route analyze and scan through app services`).
- CI run [#44849791146](https://github.com/intorbmn/Universal-Minecraft-Mod-Porter/actions/runs/44849791146) was triggered for that commit and is pending. No test success is claimed for this change until that run completes.
- Remaining 12.A work is still substantial: route plan/migrate, port, build, pack and rollback through application services; add domain result views for every supported JSON command; complete error/exit mapping and golden tests; verify all four shell completions and documentation.


## 12.A JSON envelope tests and CI correction

- Added pure success/failure JSON envelope builders and four golden-shape unit tests in `cli/src/output.rs`; commit `a935c36`.
- CI run [#44852734836](https://github.com/intorbmn/Universal-Minecraft-Mod-Porter/actions/runs/44852734836) failed only in Clippy: `success_envelope` accepted `serde_json::Value` by value without consuming it.
- Fixed the Clippy finding by borrowing the result value; commit `7cbdd8e` (`fix(cli): borrow JSON envelope result`). The new commit's CI must pass before this change is considered verified.


## 12.A utility JSON contract

- Added an integration test for `adapters --json`: validates the versioned envelope, stable result keys, and typed adapter entries.
- CI run for the test commit is required before treating it as verified.
- `remap-jar`, `adapters`, and `rollback` now expose structured JSON results; updated `docs/CLI.md` to state exactly which commands have command-specific JSON results.


## 12.A AppServices command routing

- Confirmed CI run #252 on commit `0ed95d1`: both `build-test-lint` and `desktop` passed.
- Routed CLI `port` and `build` through `AppServices` jobs. The CLI polls the job view,
  forwards log lines, and returns the job's structured result in JSON mode.
- Preserved local build risk confirmation (Medium requires confirmation; High requires
  `--i-accept-risk`) and the existing custom build report path.
- Extended `PortRequest` with optional KB/mapping overrides so CLI flags are not dropped
  by the application boundary.
- Routed `rollback` through `AppServices`, including the CLI's custom snapshot directory.
- Added CLI JSON failure-contract tests for invalid `port`, `build`, and `rollback` inputs.
- Verification pending: the CI run for this routing change must pass before these changes
  are treated as complete.
