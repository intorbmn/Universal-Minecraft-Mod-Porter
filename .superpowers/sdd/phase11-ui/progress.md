# Phase 11 ledger

- 2026-11-20: Ruling: the handed-over patch (`phase_11_ui_patch.md`) is unusable as a git patch (markdown rendering stripped generics and mangled indentation); 11.A1-A3 rebuilt by hand from its text and its test names, tests first - cost if wrong: small naming drift from the previous author's code
- 2026-11-20: Ruling: Rust 1.97 via `RUSTUP_TOOLCHAIN=stable`, not a rustup download - static.rust-lang.org blocked in sandbox - cost if wrong: clippy/fmt output may differ from pinned 1.97.0 by a patch version
- 2026-11-20: Ruling: `PlanView.files[].needs_attention` (assisted/manual adapter finding in that file) instead of per-file `low_confidence` - Replacement carries no confidence; low-confidence mappings are reported in `PlanView.skipped` - cost if wrong: UI label wording only
- 2026-11-20: Ruling: `AppServices::analyze` accepts folder or .zip only (JAR -> Input error pointing to scan) - CLI `analyze` is folder-only too - cost if wrong: add JAR analysis later in the service
- 2026-11-20: Ruling: `targets()` returns the four loader families the converter knows; `versions` come from `loader_templates.toml` only (ports to other versions stay possible via `plan`/`port`) - cost if wrong: UI offers fewer versions than the engine supports
- 2026-11-20: Mutation check (11.A3) `plan_returns_diffs_and_writes_nothing`: (1) plan writes MUTANT.txt into the input -> red at "plan must not write"; (2) plan drops all file views -> red at "files is not empty"; restored -> green.

Task A4 (jobs + events): complete. `core/src/app/jobs.rs` (CancelToken, JobCtx step/log/progress with <=20 log events/s, worker thread + catch_unwind, stale Running -> Failed on open), `start_port`, `job`, `cancel`, `subscribe`, `set_step_hook`; event payload golden JSON in tests/fixtures/golden/app-events/.
- Ruling: finish order = save job -> remove from running -> publish job.finished -> drain bus (a listener that sees finished can already read the stored job; the queue cannot grow).
- Ruling: failure text is stored in Job.output as {"error": msg}; JobView.error is derived from it.
- Mutation check (★ port_job_emits_events_in_order): (1) job.started published under another name -> RED (left "job.skipped", right "job.started"); (2) success finished as Failed -> RED at the state assertion. Restored -> 22/22 green, stable over 6 runs.
- Four commands pass; cargo test --all green (app: 22 tests).

Task A5 (in-place migrate, rollback, cancel): complete. `start_migrate_in_place(req, reviewed)` (Refused before any write when !reviewed; folder only), `rollback(project, Option<Uuid>)`, cancel restores the checkpoint at the post-apply "verify" step. `plan()` now shares `resolve_plan` with the job.
- Ruling: cancel is checked at step boundaries only (engine apply is atomic and already rolls itself back on failure); cancelling after apply restores the checkpoint and ends Cancelled.
- Mutation check: (1) ignore `reviewed` -> in_place_migrate_requires_reviewed_flag RED; (2) skip restore after cancel -> cancel_in_place_migrate_rolls_back_and_reports_cancelled RED. Restored -> 25/25, stable over 5 runs. Four commands pass.
