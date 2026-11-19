# Phase 11 — User Interface (guide for Claude Code)

Read [README.md](README.md) first. Rough history: `docs/drafts/phase-11-ui.md`.

## 0. Skills
- **Always** (README §3):
  - brainstorming → writing-plans → executing-plans + TDD
  - karpathy-guidelines
  - rust-errors / rust-idiomatic / rust-testing
- **Phase-specific:**
  - `frontend-design:frontend-design`, `ui-ux-pro-max:ui-ux-pro-max` (design)
  - `rust-cli-skills:rust-design` (the services facade)
  - `rust-cli-skills:rust-concurrency` (jobs running in the background)
  - `example-skills:webapp-testing` (Playwright tests of the web view)
  - `ecc:architecture-decision-records`
- **Finish:** verification-before-completion, `ecc:rust-review`.

## 1. Goal (master prompt, Phase 11)
A desktop application for users who do not use a terminal. It must let them:
- open a mod source or a pack;
- see the analysis and the technologies;
- choose a target version and loader;
- review the plan and the diff before anything is written;
- run the migration with progress;
- roll back;
- run the build/test and read the report;
- use the AI assistant with explicit approval of every edit;
- manage settings (KB path, AI provider key, GitHub sandbox repo, output folder).

**ADR 0004 rule:** the UI calls *application services* only, never engine internals.
Every action the UI can do must also be callable from Rust tests without the UI.

**Non-goals:**
- a web/cloud version;
- user accounts;
- auto-update (Phase 13);
- a mod editor (the user edits in their IDE).

## 2. What exists
- **Engines:**
  - `AnalyzerEngine`, `MigrationPlanner`/`MigrationEngine` (`dry_run`, checkpoints,
    `MigrationReport`)
  - `port::port`, `scan::scan_path`
  - `build::{local, github, report}`
  - `ai::orchestrator::Orchestrator` + `ApprovalGate`
  - `modpack` (Phase 10)
- **Exist since Phase 3 but are lightly used; Phase 11 makes them real:**
  - `jobs::{Job, JobState, JobStore}` (`Job::new(session_id, step)`, `log_line`,
    `mark_succeeded`)
  - `persistence::JobDb::{open, insert, by_session, delete_by_session}`
  - `session::MigrationSession::{new(project_path, SessionConfig), start, succeed, fail}`
  - `events::EventBus::{new() -> Arc<Self>, subscribe, publish(kind, payload), drain}`
- **CLI:** `cli/src/main.rs` has helpers that should move into the services layer so that
  the CLI and the UI share them: `run_build`, `check_risk`, `mod_identity`,
  `detected_loader`, `open_kb`.

## 3. Questions for the user (defaults in brackets)
1. UI stack? [Tauri 2 + a small TypeScript front end (Svelte or vanilla + Vite). It stays
   Rust-native, gives a small installer, and the Windows WebView2 is already on Windows 10
   and 11.]
2. Language of the UI text? [Vietnamese and English, with a switch; strings in a JSON
   catalog]
3. Which screens are must-have for the first release? [Home/Open, Analysis, Plan + Diff,
   Run + Progress, Report, Build, Settings. AI chat and Pack screens second.]
4. Where are secrets stored? [OS keyring through the `keyring` crate; never in plain-text
   config]

## 4. Architecture

### 4.1 Milestone 11.A first: application services (no UI yet)
New module `core/src/app/` (the facade from ADR 0004):

```rust
pub struct AppServices { kb_path: PathBuf, jobs: JobDb, events: Arc<EventBus>, /* settings */ }

impl AppServices {
    pub fn open(settings: Settings) -> Result<Self>;
    pub fn analyze(&self, input: &Path) -> Result<AnalysisView>;
    pub fn plan(&self, input: &Path, target: Target) -> Result<PlanView>;          // dry run, includes diff
    pub fn migrate(&self, input: &Path, target: Target) -> Result<JobId>;          // background job
    pub fn port(&self, opts: PortRequest) -> Result<JobId>;
    pub fn build(&self, dir: &Path, backend: BackendChoice) -> Result<JobId>;
    pub fn rollback(&self, session: SessionId) -> Result<()>;
    pub fn job(&self, id: JobId) -> Result<JobView>;                              // state + log tail
    pub fn cancel(&self, id: JobId) -> Result<()>;
    pub fn sessions(&self) -> Result<Vec<SessionView>>;                           // history
    pub fn subscribe(&self) -> Receiver<AppEvent>;                                // progress events
}
```
- **`*View` types** are plain `serde::Serialize` structs: everything the screen shows, and
  nothing the screen must compute.
- **`AppEvent`:**
  `{ JobStarted, Progress { job, step, pct }, Log { job, line }, JobFinished { job, ok }, ApprovalNeeded { job, edits } }`.
- **Background jobs:**
  - run on `std::thread`, recorded in `JobDb`;
  - cancellation uses an `AtomicBool` checked between steps;
  - build processes are already cancellable through `build::process`.
- **AI approval:** the existing `ApprovalGate` becomes a channel. The job emits
  `ApprovalNeeded` and blocks until `approve(job, decision)`. Test it with a fake gate.
- **CLI refactor:** the CLI calls `AppServices` too. The output of existing CLI tests must
  not change; that is how you prove the refactor.

### 4.2 Tauri app (milestones 11.B onwards)
- **Layout:**
  - `ui/` workspace member: `ui/src-tauri/` (Rust) and `ui/src/` (front end).
  - Add it to the workspace `members`.
  - CI must still pass on Linux: build the Tauri crate with `--no-default-features`, or put
    the Tauri build in its own CI job with the system libs. Decide and write it in the ADR.
- **Tauri commands** are thin wrappers (one line each) over `AppServices`.
- **Events** are forwarded with `app_handle.emit("ummp://event", payload)`.
- **Front end:** no business logic, only views and forms. A diff viewer component shows the
  `FileEdit` original/replacement. Use a small diff library or a hand-rolled line diff;
  name the choice in the design doc.
- **Security:**
  - Tauri capabilities: allow only the UMMP commands and the dialog plugin. No shell plugin,
    no HTTP from the web view.
  - The CSP is strict and allows no remote content.
- **Paths from the dialog** still go through the services, and the services validate them.

### 4.3 Screens (wireframe them as text in the design doc; the user approves the look)
1. **Home:** open a folder/zip/pack, recent sessions, a drag & drop area.
2. **Analysis:** mods, loader, version, technologies (adapter findings), risk badge from
   the scan.
3. **Target:** Minecraft version + loader pickers. The options come from KB/templates data,
   not from a hard-coded list.
4. **Plan & Diff:** stages, confidence, a file tree with a per-file diff. Low-confidence
   edits are flagged. "Apply" is disabled until the user ticks "I reviewed the changes".
5. **Run:** a progress bar per step, a live log, a cancel button.
6. **Report:** `MigrationReport` rendered, with buttons for "Open output folder",
   "Build", "Rollback".
7. **Build:** backend choice (local / GitHub), stages, score, failed-log tail.
8. **AI assistant:** chat, proposed `FileEdit`s with Approve/Reject per edit, a cost
   counter.
9. **Pack** (when Phase 10 is merged): the pack table with action and reason per mod.
10. **Settings:** KB path, output folder (default `D:\Claude\…`), AI provider + key
    (keyring), GitHub sandbox repo, language.

## 5. Milestones

### 11.A Services facade
**Tests** in `core/tests/app.rs`:
- ★ `migrate_job_reports_progress_events_in_order`
- ★ `plan_returns_diff_without_writing`: hash the tree before and after.
- `rollback_restores_tree`
- `cancel_stops_job_and_marks_cancelled`
- `job_survives_restart_via_jobdb`
- `ai_edit_waits_for_approval_and_reject_writes_nothing`
- `settings_never_contain_secret_in_plaintext`

Also, all existing CLI tests stay green after the CLI is moved onto the services.

### 11.B Tauri shell + Home + Analysis
- Rust unit tests for the command wrappers (call them with a mock `AppServices`, or with
  the real one over fixtures).
- A Playwright test against `tauri dev`, or the front end served on its own with a mocked
  `invoke`: open the fixture folder → the analysis table shows the mod id.

### 11.C Target + Plan & Diff + Run + Report
- **E2E:** open `tests/fixtures/loader/fabric-src` → target NeoForge 1.21.1 → plan shows
  the diff → apply is disabled until reviewed → run → the report shows the stage → rollback
  restores the tree.

### 11.D Build + Settings (keyring) + i18n
- **Tests:**
  - `secret_round_trips_through_keyring_mock`
  - the language switch changes the labels
  - a missing key in one language falls back to English

### 11.E AI assistant + Pack screen
- **E2E with a fake provider:** a proposed edit appears; reject leaves the files
  unchanged; approve applies it through the engine (checkpoint present).

### 11.F Packaging check
- `cargo tauri build` on Windows, through a GitHub Actions `windows-latest` job in the
  sandbox repo or the main repo CI as a manual workflow. Upload the `.msi`/`.exe` as an
  artifact. The artifact download is blocked here: report the run id and let the user
  download it.
- The user installs it on their machine. The UMMP real-world check is the user's
  confirmation that it opens and ports the demo mod.

## 6. Design quality (the user will look at it)
- Follow `frontend-design` and `ui-ux-pro-max`.
- Choose a clear, calm style: dark and light themes, readable diff colors, keyboard
  shortcuts for approve/reject, empty states with a next action.
- Show 2–3 mock directions first (text sketches or the visual companion) and let the user
  pick one before building screens.
- Accessibility:
  - every control has a label;
  - focus is visible;
  - colors pass contrast checks;
  - meaning is never carried by color alone (also use icons or text for risk and diff).

## 7. Docs (same PR)
- **ADR 0014** (or the next number): UI stack, services facade, secret storage.
- `docs/phase/phase-11-ui.md`, STATUS, CHANGELOG, README (screenshots: store PNGs under
  `docs/images/`), CLAUDE.md layout (`ui/`, `core/src/app/`), KNOWN_LIMITATIONS, BACKLOG.

## 8. Risks
- **Tauri system deps on Linux CI** (webkit2gtk, libsoup): install them in a separate CI
  job, or keep `ui/` out of the default `cargo test --all`. Choose explicitly; never let the
  core CI go red.
- **Long jobs blocking the UI thread:** every service that may take more than 100 ms
  returns a `JobId`.
- **Divergence between the CLI and the UI:** prevented by both calling `AppServices`.

## 9. Backlog candidates
- macOS signing/notarization
- Linux AppImage
- auto-update
- a project-wide search in the diff view
- a dark/light theme editor

## 10. Definition of done
README §7, plus:
- the services facade is tested;
- the CLI runs on the facade;
- the E2E flow passes;
- a Windows installer is built in CI and confirmed by the user.
