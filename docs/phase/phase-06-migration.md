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
| Status | PLANNED |