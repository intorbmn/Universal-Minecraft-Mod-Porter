# Phase 14 — Community & Ecosystem

**Status: IN_PROGRESS.** 14.A and 14.B are implemented on branch `phase-14ab-contributor-sdk`
and wait for the maintainer's review. 14.C–14.H are not started.
Guide: `docs/guides/phase-14-community.md`.

## Decisions by the maintainer (2026-11-27)

| Question | Answer |
|---|---|
| Community data license | **CC0** (code stays Apache-2.0, ADR 0018) |
| Where the community KB lives | separate public repository `ummp-knowledge`, created only after the maintainer agrees |
| Who reviews contributions | the maintainer, with UMMP's automatic validation; no automatic merges |
| Documentation website | mdBook built from `docs/`, published with GitHub Pages |
| Protocol types in the SDK | two definitions plus golden JSON (ADR 0022) |

Phase 13 is still IN_PROGRESS: the RC tag could not be pushed from the agent sandbox (HTTP 403
from the egress proxy), so the maintainer pushes `v1.0.0-rc.1`. Phase 14 started on the
maintainer's explicit instruction; 14.E (plugin install from release assets) and 14.G
(nightly regression runs) need published releases and stay blocked until the RC exists.

## Pull request plan

| PR | Milestones | State |
|---|---|---|
| 1 | 14.A contributor docs, 14.B plugin SDK | implemented, awaiting review |
| 2 | 14.C knowledge base, 14.D recipes | not started |
| 3 | 14.E plugin index, 14.F issue reporting | not started |
| 4 | 14.G community regression, 14.H docs website | not started |

## 14.A Contributor docs

Delivered: `CONTRIBUTING.md` (setup, the four commands, TDD and mutation rule, real-format
fixtures, synthetic names, ADR process, PR flow), `.github/PULL_REQUEST_TEMPLATE.md`, issue forms
(bug, port request, rule contribution with a CC0 consent box) and `CODE_OF_CONDUCT.md`
(Contributor Covenant 2.1 by reference). The old `CONTRIBUTING.md` had a wrong repository URL and
a false line-limit rule; both are gone.

Verification: the commands written in the tutorial were run (see 14.B). Clean-clone review
(2026-11-27, done by the same agent that wrote the file, so not independent): a fresh `git clone`
of the branch from GitHub, then `cargo build --all` (1m18s), `cargo fmt --all -- --check`, the SDK
and template tests and `core/tests/plugin_template.rs` all passed exactly as `CONTRIBUTING.md`
says. An independent follow-through by the maintainer is still wanted.

## 14.B Plugin SDK

Delivered: crate `plugin-sdk/` (`ummp-plugin-sdk`), template `templates/plugin/` (`demo-renamer`),
golden files in `tests/fixtures/golden/plugin-protocol/`, host request builders in
`core/src/plugins.rs`, rewritten `docs/PLUGIN_SDK.md` (folds in the adapter notes; `ADAPTER_SDK.md`
is now a pointer), ADR 0022.

### Evidence

| Test | Result |
|---|---|
| `ummp-plugin-sdk` suite (7 tests, `plugin-sdk/tests/protocol.rs`) | pass |
| core `plugins::tests::core_requests_match_golden_protocol_files`, `core_parses_golden_plugin_responses` | pass |
| ★ `core/tests/plugin_template.rs` builds the template, `PluginManager::check`, applies its edit through `MigrationEngine`, rolls back | pass |
| template unit tests (3) | pass |

### Mutation checks (each test went red, then the source was restored)

| Mutation | Test that failed |
|---|---|
| SDK `Hello.ummp_api` renamed to `api` | `sdk_responses_equal_golden_core_responses` |
| core `edits_request` key `target` renamed to `targets` | `core_requests_match_golden_protocol_files` |
| template `TO` changed to another name | `template_plugin_passes_check_and_its_edit_is_applied_by_the_engine` (left/right mismatch) |
| template `id` differs from the manifest name | same test: "hello id does not match manifest name" |

## CI finding

The first CI run of PR #34 failed in `cargo deny check` (`bans`: wildcard dependency for
`ummp-plugin-template`, because its path dependency on the SDK had no `version`). Fixed by adding
`version = "1.0.0-rc.1"`, the same pattern `cli/Cargo.toml` uses for `ummp-core`. Lesson: new
workspace crates need versioned path dependencies; `cargo deny` is not installed in the sandbox,
so only CI catches this.

## Open items

- Clean-clone review of `CONTRIBUTING.md` (14.A) and the maintainer's review of all user-facing
  text (definition of done).
- 14.C–14.H.
