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
| 12.0 AI credentials | IMPLEMENTATION COMPLETE; ACCEPTANCE PENDING | File store, desktop UI, CLI commands, legacy keyring migration and automated leak/write-failure tests. CI #288 green; real-machine sharing check remains. |
| 12.A CLI polish | IMPLEMENTATION COMPLETE; ACCEPTANCE PENDING | Exit codes, global JSON envelopes, strict config precedence, completions and AppServices routing for core workflows. CI #288 green; golden/mutation and real-shell checks remain. |
| 12.B REST API | NOT STARTED; GATED | Separate `server/` workspace member; localhost + bearer token. Start only after PR 1 is reviewed and merged. |
| 12.C MCP server | NOT STARTED; GATED | stdio transport; every write must use safe application services and explicit approval. |
| 12.D GitHub Action | NOT STARTED; GATED | Composite action; build from source until Phase 13 release binaries exist. |
| 12.E Plugin interface | NOT STARTED; GATED | Out-of-process JSON-lines protocol; plugins return edits and never write project files. |

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

1. Finish golden fixtures for every command-specific JSON result shape; current tests pin the envelope and representative command results.
2. Verify completion installation/behavior in real bash, zsh, fish and PowerShell sessions.
3. Have the user confirm on their machine that a key saved in desktop appears as masked status in CLI, and vice versa.
4. Run and record mutation checks for the starred credential, leak, JSON and exit-code tests.
5. Review and merge PR 1 before starting 12.B or 12.C.

## 12.A JSON coverage follow-up

- Added structured JSON results for `pack inspect`, `pack export`, `ai-assist` and `ai-debug`; all commands except shell completion generation now produce a versioned result or error envelope.
- Added integration coverage for `pack inspect --json` and the missing-AI-key refusal envelope.
- Added the six stable exit-code meanings to top-level `ummp --help`.
- Corrected ADR 0015's stale historical note: ADR 0015 is the interim AI credential decision; the CLI contract remains in `docs/CLI.md`.
- CI verification is pending for this follow-up.

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


### 12.A AppServices routing follow-up

- Routed `migrate` planning through `AppServices::plan`; `--apply` now requires the
  explicit flag and runs `start_migrate_in_place` as a tracked job after producing the
  review plan.
- Extended `PlanRequest` with backward-compatible optional source-loader, KB, mappings
  and snapshot overrides. Desktop callers remain compatible through serde defaults.
- Routed `pack migrate` through `AppServices::start_pack`, retaining online/offline
  index selection, output options, KB/mappings overrides and pack exit codes.
- Added regression coverage for request-level mapping overrides and the CLI invalid-input
  JSON contract. CI verification is pending for this commit.


## PR 1 verification follow-up

- Added a core integration test that resolves a synthetic AI key from `credentials.json`
  and asserts it never appears in settings, the jobs database, job views, job logs, or events.
- Strengthened the AI screen test to verify the masked key appears after save and the raw value
  never enters the rendered document.
- Added a dedicated Tauri Cargo cache to CI; the previous desktop job had no Rust cache and
  rebuilt the Tauri dependency graph on every run.
- Corrected the implementation plan and project docs to use ADR 0015 for interim AI storage;
  ADR 0016 remains reserved for REST.
- CI verification for this follow-up is pending. The only remaining PR 1 acceptance outside
  automation is the user's real-machine check that a key saved in desktop is visible as masked
  status to the CLI (and vice versa).


## 12.A exit/config integration tests — pending CI

- Added an integration test proving CLI provider flags override `UMMP_AI_PROVIDER` and
  TOML, while the environment overrides TOML when no flag is supplied.
- Added a local Gradle failure test for exit code 4 and the versioned JSON result envelope.


## PR 1 verification update — 2026-11-21

- The shared credentials implementation includes atomic file replacement, private Unix
  permissions, key masking, environment-first resolution, schema validation, idempotent
  deletion, OpenAI base-URL validation and legacy keyring migration.
- Added regressions proving failed writes preserve the previous credential file and
  unsupported schema versions fail without exposing the version value.
- CLI integration coverage now verifies stdin key entry, rejection of command-line key
  arguments, provider-specific deletion, and file-backed key resolution by `ai-debug`
  through a local HTTP mock. The mock is isolated from external networking and writes its
  snapshots only into the test's temporary directory.
- CI run [#288](https://github.com/intorbmn/Universal-Minecraft-Mod-Porter/actions/runs/44867212630)
  on commit `9660301`: `build-test-lint` and `desktop` both passed, including Rust
  build/tests/Clippy/rustfmt and desktop npm checks, Vitest, Playwright, Tauri build/tests,
  Clippy and formatting.
- PR 1 is the 12.0 + 12.A review batch. PR 2 (12.B + 12.C) and PR 3 (12.D + 12.E) remain
  gated on the preceding PR being reviewed and merged by the user.
- Outstanding manual acceptance: real desktop↔CLI credential sharing, actual shell
  completion behavior, and mutation-check evidence for starred tests. Do not mark Phase 12
  complete until these and the later PR batches satisfy the guide's definition of done.

- Opened [PR #14](https://github.com/intorbmn/Universal-Minecraft-Mod-Porter/pull/14) for review on 2026-11-21. It is mergeable and remains unmerged; wait for the user's review and merge before starting PR 2.

## Latest PR 1 checkpoint — 2026-11-21

- Current branch head: `8640cc87df05fe40f7378816c6b966def121b46a`.
- Open PR: [#14](https://github.com/intorbmn/Universal-Minecraft-Mod-Porter/pull/14),
  covering 12.0 + 12.A only; it remains open and unmerged.
- Latest restored-code CI [#386](https://github.com/intorbmn/Universal-Minecraft-Mod-Porter/actions/runs/44875166010)
  and current-head CI [#390](https://github.com/intorbmn/Universal-Minecraft-Mod-Porter/actions/runs/44875444029)
  both passed `build-test-lint` and `desktop`.
- Desktop setup uses core `default_data_dir()`, the same resolver as CLI, and honors
  `UMMP_DATA_DIR`.
- Golden JSON coverage now includes the shared success/error envelopes, nested `scan`
  result shape, `analyze` view, adapter entries, pack inspect result, migrate PlanView,
  and `ai-key set/status/delete` result shapes. The CLI tests compare nested object keys;
  scan, adapter and pack/analyze/migrate tests use isolated temporary data/config folders.
- Completion generation checks Bash, Zsh, Fish and PowerShell registrations. Bash also
  sources the generated script and verifies command registration. Interactive suggestions
  in real shells remain a manual acceptance item.
- Mutation evidence:
  - CI [#345](https://github.com/intorbmn/Universal-Minecraft-Mod-Porter/actions/runs/44872815526):
    JSON envelope command and InvalidInput exit-code mutations caused their golden/table
    tests to fail; restored CI #348 passed.
  - CI [#380](https://github.com/intorbmn/Universal-Minecraft-Mod-Porter/actions/runs/44874839367):
    returning the raw credential instead of its mask caused
    `set_reads_stdin_and_status_only_shows_a_masked_key` to fail.
  - CI [#382](https://github.com/intorbmn/Universal-Minecraft-Mod-Porter/actions/runs/44874979017):
    ignoring the environment override caused
    `environment_value_overrides_file_without_global_environment_mutation` to fail;
    skipping legacy keyring deletion caused
    `legacy_ai_keyring_entry_moves_to_shared_store_once` to fail.
  - All temporary mutations were restored. CI #386 passed on restored code.
- Remaining acceptance before PR 1 can be closed:
  1. Add golden fixtures for remaining success payloads: `port`, `build`, `rollback`,
     `pack migrate/export`, `remap-jar`, and successful `ai-assist/ai-debug` runs.
  2. Verify interactive completion behavior in real Zsh, Fish and PowerShell sessions.
  3. User confirms on their machine that desktop-saved credentials appear as masked CLI
     status, and CLI-saved credentials appear masked in desktop.
  4. Mutation-check the remaining starred round-trip and key-leak invariants, especially
     `file_resolved_ai_key_never_enters_settings_jobs_logs_or_events`.
- **Gate:** do not begin 12.B/12.C until PR #14 is reviewed and merged by the user. Do not
  merge automatically. Preserve the existing branch history; no rebase or force-push.
