# Phase 11 — Desktop UI (Tauri)

**Status:** IN PROGRESS  
**Branch:** `phase-11-ui`  
**Design:** approved; `docs/superpowers/specs/2026-11-20-phase11-ui-design.md`  
**Plan:** `docs/superpowers/plans/2026-11-20-phase11-ui.md`  
**Handoff:** `docs/guides/phase-11b-handoff.md`

## Scope and architecture

Deliver the ten-screen desktop workflow using Svelte 5, TypeScript, Vite and Tauri 2. The UI is a presentation layer only:

`ui/src` → `ui/src-tauri` → `core/src/app/` → existing engines.

Only `ui/src/lib/api.ts` calls Tauri `invoke`. Core owns migration, planning, risk checks, jobs, cancellation, approvals, rollback, build and pack behavior. Secrets stay in the OS keyring and are never returned to the web view. In-place migration requires an explicit review acknowledgement.

## Skill availability and manual practice

The required Claude Code skills are not available in this ChatGPT session. The user explicitly approved continuing without installing them. Their practices are applied manually:

- Brainstorming and writing-plans: use the approved design and implementation plan as the scope boundary.
- Executing-plans and TDD: work from the task ledger; add or inspect tests before changing behavior.
- Systematic debugging and verification: trace failures to their source and do not mark work complete without recorded evidence.
- Karpathy guidelines: prefer the smallest implementation that satisfies the contract; avoid speculative abstractions and dependencies.
- Frontend design and UI/UX: retain the approved Calm workbench, keyboard accessibility, readable diffs, responsive layouts and Vietnamese/English fallback.
- Rust errors, idioms, testing, design and concurrency: keep typed errors, thin command adapters, trait-based boundaries and explicit cancellation/approval behavior.
- Web app testing: use Vitest/Testing Library and Playwright for user-visible flows.
- ADR and Rust review: record architectural choices in ADR 0014 and review security, ownership and error paths manually.
- PR/release workflow: inspect the final diff and CI evidence before opening a PR; do not merge.

## Milestone status

| Milestone | Implementation | Verification |
|---|---|---|
| 11.A Application services A1–A8 | Complete | Historical Rust build/test/clippy/fmt and mutation evidence recorded in the SDD ledger |
| B0 Sessions | Complete | Historical Rust test evidence recorded in the SDD ledger |
| 11.B Desktop shell, Home, Analysis | Complete | Historical Rust, Tauri, Vitest and Playwright evidence recorded in the SDD ledger; CI still needs a fresh run |
| 11.C Target, Plan/Diff, Run, Report | Implemented | Playwright workflow and keyboard-focus tests exist; fresh CI run pending |
| 11.D Build and Settings | Implemented | Keyring and risk-gate tests exist; fresh CI run pending |
| 11.E AI assistant and Pack | Implemented | Approval and 300-row pagination/performance tests exist; fresh CI run pending |
| 11.F Windows packaging | Workflow added | Windows workflow has not yet been dispatched; real Windows install/smoke test remains outstanding |
| 11.G Documentation and PR | In progress | ADR 0014 exists; phase/status/limitations/backlog/architecture docs are being aligned; PR not opened |

## Test and acceptance commands

Run from the repository root unless noted:

```sh
export RUSTUP_TOOLCHAIN=stable
cargo build --all
cargo test --all
cargo clippy --all --all-targets -- -D warnings
cargo fmt --all -- --check
cd ui
npm ci
npm run check
npx vitest run
npx playwright test
npm run build
cd src-tauri
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

The root Rust toolchain must not be changed to bypass the pin. The Tauri build requires the frontend assets and Linux system libraries described in the handoff. A CI run on GitHub is required because this session has no local checkout/runtime in which to execute these commands.

## Outstanding acceptance gates

1. Run the full CI workflow on `phase-11-ui`; fix every failure and rerun until green.
2. Dispatch `.github/workflows/desktop-release.yml`; verify MSI and NSIS artifacts are produced.
3. On Windows 10/11, install and launch the app, then run the agreed sample-mod workflow and verify output, build/report and rollback behavior.
4. Inspect the final diff and CI logs, then open a PR targeting `master`. Do not merge and do not start Phase 12.

## Known limitations

- Build cancellation is honored between steps; an active Gradle or GitHub Actions process is not interrupted.
- The Windows installer is unsigned.
- A real Windows install and smoke test cannot be replaced by mocked browser tests.
- The Tauri window was not launched in the original headless sandbox; the real desktop runtime remains to be verified.
- Tauri default icons remain in use.

