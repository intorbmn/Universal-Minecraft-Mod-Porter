# Phase 11 — User Interface (Tauri): design

Status: **visual direction picked (A, Calm workbench, 2026-11-20); waiting for the user's approval of this doc.**
Source of truth for scope: `docs/guides/phase-11-ui.md`. This file records the calls made on top of it.
The user answered "cứ làm theo kế hoạch" on 2026-11-20, so the guide's defaults below are my calls, not new requirements.

## Intent
People who do not use a terminal need to port or migrate a mod or pack: open it, see what UMMP found,
pick a target, **review the diff first**, run with visible progress, read the report, build, and roll back.
Every action is an application service in `core/src/app/`; the UI contains no migration logic (ADR 0004).

## Goals
- Ten screens: Home, Analysis, Target, Plan & Diff, Run, Report, Build, AI assistant, Pack, Settings.
- `core/src/app/` facade (`AppServices`) with Rust tests; the CLI is moved onto the same functions (existing CLI tests unchanged).
- Long work runs as jobs on worker threads; progress, log and approvals go through `EventBus` to the window.
- Secrets (AI key, GitHub token) live in the OS keyring only; no `get_secret` command exists.
- Windows installer (`.msi`/`.exe`) built by a `workflow_dispatch` job on `windows-latest`.

## Non-goals
Auto-update, code signing, macOS/Linux bundles (Phase 13); REST/MCP (Phase 12); editing mod source in the app;
web/cloud version; accounts and telemetry. Requests for these go to `docs/BACKLOG.md`.

## Anti-goals
- A UI-side file write (files change only through `MigrationEngine` after a checkpoint, or into a new output folder).
- An in-place apply without the explicit "I reviewed the changes" confirmation.
- A job stuck in "Running" forever; an AI edit that is approved by default; a secret in settings, logs or events.
- Business logic or an event contract that exists only in TypeScript.

## Approaches
1. **`core/src/app/` facade + `ui/src-tauri` as a separate Cargo workspace + Svelte front-end (picked).** Root CI stays
   unchanged and green without webkit libraries; a second CI job builds the desktop crate.
2. `ui/src-tauri` as a member of the root workspace: `cargo build --all` would need webkit2gtk/libsoup on every machine
   and in the sandbox. Rejected.
3. Put use-case code in the Tauri crate: not testable without a display, and the CLI could not share it. Rejected (ADR 0004).

## Calls made (defaults from the guide §3; each is cheap to change before 11.B)
1. **Front-end:** Svelte 5 + Vite + TypeScript, Vitest + Testing Library, Playwright. Cost if wrong: screens are thin; the facade and `api.ts` stay.
2. **Language:** Vietnamese + English with a switch, English fallback; strings in `ui/src/lib/i18n/{vi,en}.json`.
3. **Screens:** all ten. Cut order if the phase grows: AI assistant, then Pack, then Build (each cut goes to BACKLOG).
4. **Workspace placement:** `exclude = ["ui/src-tauri"]` in the root `Cargo.toml`, own `Cargo.lock`, CI job `desktop`.
5. **Installer:** `.github/workflows/desktop-release.yml` (`workflow_dispatch`, `windows-latest`); the user downloads the artifact (blocked in this sandbox).
6. **Events:** kinds `job.started|progress|log|approval|finished`, golden JSON in `tests/fixtures/golden/app-events/` shared by Rust and TypeScript tests.
7. **Jobs:** `std::thread` per job, `CancelToken`, `catch_unwind`, new `JobState::Cancelled`, stale `Running` jobs marked `Failed` on open. No async runtime in `core`.
8. **Environment:** built here with Rust 1.97 from apt (static.rust-lang.org is blocked); 213 tests pass on `master` as the baseline.

## Design
Layers: `ui/src` (Svelte, `invoke` only in `lib/api.ts`) → `ui/src-tauri` (one-line commands, keyring, event forwarder)
→ `core/src/app/` (`mod.rs`, `views.rs`, `jobs.rs`, `approval.rs`, `snapshots.rs`, `helpers.rs`) → existing engines.
Signatures and event payloads follow guide §4.2–§4.4. CLI helpers (`open_kb`, `check_risk`, `mod_identity`, `detected_loader`,
`run_build`, `default_snapshots`, `load_store`, `save_store`, `side`, `pack_format`) move into `core/src/app/` in 11.A.
Capabilities: dialog open + opener only (no `shell`, `fs`, `http`). CSP as in guide §4.5.

## Testing
Guide §7 test names, test-first. ★ tests are mutation-checked and recorded in the phase file.
11.A is Rust-only (`core/tests/app.rs`); 11.B–E add Vitest and mocked-IPC Playwright; 11.F is the real Windows install by the user.
Honest limit: this sandbox has no display and no Windows, so the real-binary check is the user's.

## Visual directions (user picked **A. Calm workbench** on 2026-11-20)
Common to all: light and dark theme following the OS, WCAG AA diff colors, `+`/`−` markers (never color alone),
system font stack (no remote fonts), Vietnamese strings must fit.

**A. Calm workbench** — neutral greys, one blue accent, dense tables, 13–14 px UI font, monospace only in diffs.
Fits power users who port many mods; most information per screen.
```
┌ UMMP ── DemoWidget ▸ Plan ───────────────────────── [vi|en] [⚙] ┐
│ 1 Open ✓ 2 Analyze ✓ 3 Target ✓ 4 Plan ● 5 Run  6 Report       │
├──────────────┬───────────────────────────────────────────────────┤
│ ▾ src (12)   │ src/.../DemoWidget.java            3 changes  ⚠ 1│
│   M DemoW…   │  12 │ - import net.demo.OldItem;                 │
│   M Demo…    │  12 │ + import net.demo.NewItem;                 │
│ ▾ resources  │  …  │ ⋯ 40 unchanged lines                       │
│   M fabric…  │                                                   │
│   + neofo…   │  ☐ I reviewed the changes      [ Apply (disabled)]│
└──────────────┴───────────────────────────────────────────────────┘
```

**B. Guided wizard** — large single-purpose steps, generous whitespace, 15–16 px font, one primary button per page.
Fits first-time, non-technical users; fewer things on screen, more clicks.
```
        ●────●────●────◉────○────○       Step 4 of 6: Review the changes
                                         
   We will change 14 files (2 need a look).
   ┌──────────────────────────────────────────────┐
   │ DemoWidget.java      3 changes      [ View ▸ ]│
   │ fabric.mod.json → neoforge.mods.toml  [ View ▸ ]│
   └──────────────────────────────────────────────┘
   ☐ I reviewed the changes               [ Start porting ]
```

**C. IDE-like** — split panes (tree | diff | log/inspector), mostly monospace, status bar with job state, keyboard-first.
Fits developers who live in editors; steepest for beginners.
```
┌ Files ───────┬ Diff ────────────────────────┬ Inspector ────────┐
│ M DemoW…     │ @@ -12,3 +12,3 @@              │ Technologies      │
│ M fabric…    │ -import net.demo.OldItem;      │  Mixin ×2         │
│ + neoforge…  │ +import net.demo.NewItem;      │ Risk: Low         │
├──────────────┴───────────────────────────────┴───────────────────┤
│ ● Plan ready · 14 files · 2 low confidence      [Apply ⌘↵]       │
└──────────────────────────────────────────────────────────────────┘
```

## Left to the builder
Component names, spacing and exact copy, internal helper names, exact `--help` text.
