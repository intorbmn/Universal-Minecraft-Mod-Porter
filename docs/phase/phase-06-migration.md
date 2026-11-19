# Phase 6 — Migration Engine

| Field | Value |
|---|---|
| Phase ID | P6 |
| Objective | Execute the migration pipeline: plan, transform source/build/resources/mixins, produce diffs, rollback. |
| Scope | Version migration, loader migration, source transformation, build migration, metadata/resource/datapack/mixin/bytecode migration, diff viewer, migration report, rollback, regression tests. |
| Prerequisites | Phase 4 (analysis) + Phase 5 (recipes). |
| Modules | ENG-03. |
| Architecture | Pipeline steps behind interfaces; each step checkpointable and resumable. |
| Tasks | 6.1 Planner: build change graph from analysis + recipes 6.2 Source transformer 6.3 Build config transformer 6.4 Resource/datapack transformer 6.5 Mixin transformer 6.6 Bytecode transformer (adapter) 6.7 Diff viewer 6.8 Migration report 6.9 Rollback implementation 6.10 Regression tests on corpus |
| Dependencies | P4, P5. |
| Deliverables | Working migration pipeline, diff viewer, report, rollback, regression tests. |
| Tests | Integration: run full pipeline on corpus mods; assert transforms produce valid output. |
| Risks | Transform breaks compilation; recipe coverage gaps. |
| Acceptance Criteria | A corpus mod migrates through transform step and produces a valid diff; rollback restores original exactly. |
| Complexity | L |
| Status | IN_PROGRESS |

## Milestones
| ID | Milestone | Status | Evidence |
|---|---|---|---|
| 6.A | Planner + Java/Kotlin API-rename transformer + diff + report + checkpointed apply + rollback + CLI | COMPLETED | `core/tests/migration.rs` (golden file, stage order, conflicts, downgrade rejection, rollback after simulated write failure); 8 tokenizer unit tests; CLI end-to-end run (dry run → apply → rollback, `diff -r` identical) |
| 6.B | Descriptor migration (`mods.toml`, `neoforge.mods.toml`, `fabric.mod.json`, `quilt.mod.json`) | COMPLETED | `core/src/descriptor.rs` (8 unit tests: style-preserving ranges, formatting kept, loader version from KB, mismatch skip, NeoForge rename warning); fixture `mods.toml` vs golden in `core/tests/migration.rs`; mutation-checked |
| 6.C | Build migration (Gradle plugin/loader/mappings versions, Java toolchain) | NOT_STARTED | |
| 6.D | Resource / datapack migration | NOT_STARTED | |
| 6.E | Mixin migration | NOT_STARTED | |
| 6.F | Bytecode migration (adapter over an external tool) | NOT_STARTED | |
| 6.G | Recipe execution (interpret `Recipe.steps`) | NOT_STARTED | |

## Milestone 6.A report
- **Implemented:** `MigrationPlanner` (mappings in `(source, target]`, per-version stages,
  confidence threshold, conflict/no-op detection, breaking changes, recipes);
  `SourceTransformer` trait + `TokenRenameTransformer` (ADR 0007); `FileEdit` + unified diff;
  `MigrationReport` (Markdown/JSON); `MigrationEngine::run` with checkpoint-before-write and
  automatic restore on write failure (ADR 0005/0006); CLI `migrate` / `rollback`.
- **Architecture changes:** ADR 0006 (snapshot checkpoints), ADR 0007 (token-aware rewrite).
  Phase 3 checkpoint gained real `snapshot`/`restore`.
- **Known issues / technical debt:** see `docs/KNOWN_LIMITATIONS.md` → Migration engine.
- **Risks still open:** R1/R4 (no real mapping dataset yet), R5 (Mixins untouched).
- **Entry condition for 6.B:** user review of ADR 0007 rewrite policy (`should_rewrite`).