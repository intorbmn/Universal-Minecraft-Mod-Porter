# Phase 11 ledger

- 2026-11-20: Ruling: Rust 1.97 from apt, not rustup — static.rust-lang.org blocked in sandbox — cost if wrong: clippy/fmt output may differ from pinned 1.97.0 by a patch version
- 2026-11-20: Ruling: local git identity "Claude <noreply@anthropic.com>" for commits in the sandbox — no identity configured — cost if wrong: rewrite author before PR
- 2026-11-20: Mutation check (11.A3) `plan_returns_diffs_and_writes_nothing`: (1) plan writes MUTANT.txt into the input -> red at "plan must not write"; (2) plan drops all file views -> red at "files is not empty"; restored -> green.
- 2026-11-20: Ruling: `PlanView.files[].needs_attention` (assisted/manual adapter finding in that file) instead of per-file `low_confidence` — Replacement carries no confidence; low-confidence mappings are reported in `PlanView.skipped` — cost if wrong: UI label wording only
- 2026-11-20: Ruling: `AppServices::analyze` accepts folder or .zip only (JAR -> Input error pointing to scan) — CLI `analyze` is folder-only too — cost if wrong: add JAR analysis later in the service
- 2026-11-20: Ruling: `targets()` versions come from `loader_templates.toml` only; ports to other versions stay possible via `plan`/`port` — cost if wrong: UI offers fewer versions than the engine supports
