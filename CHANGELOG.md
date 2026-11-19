# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Phase 6.C–6.G: build-script migration (`build_script.rs`), resources and 1.21 data pack folder
  moves (`resources.rs`, KB `pack_formats` table), Mixin migration (`mixin.rs`), class-level
  bytecode remapping (`bytecode.rs`, `ummp remap-jar`, ADR 0009), and recipe execution.
- `FileSystem::remove`; `MigrationEngine::compute_edits` now returns an `EditSet` with moves.
- Phase 6.B descriptor migration (`core/src/descriptor.rs`): retargets Minecraft and loader
  version requirements in Fabric/Quilt/Forge/NeoForge descriptors, preserving formatting.
- ADR 0008: simple names after an all-lowercase (package) qualifier are renamed.
- Phase 6.A migration engine (`core/src/migration.rs`): planner with per-version stages,
  token-aware Java/Kotlin API renamer, unified diffs, Markdown/JSON report, checkpointed apply
  with automatic restore on failure.
- CLI: `ummp migrate` (dry run by default, `--apply`, `--report`), `ummp rollback`;
  `ummp analyze` now runs the analyzer engine (`--json` for machine-readable output).
- Real checkpoint snapshots and restore (`CheckpointStore::snapshot` / `restore`), and
  `filesystem::resolve_within` path guard.
- Analyzer: Quilt (`quilt.mod.json`), NeoForge `neoforge.mods.toml`, `[[dependencies.<modId>]]`
  tables and version ranges.
- ADRs 0001-0007 in `docs/decisions/`; `docs/KNOWN_LIMITATIONS.md`.

### Fixed
- Analyzer aborted on any JAR containing a binary entry (all real mods).
- `parse_version("1.21")` returned 1.21.1.
- All knowledge-base tests failed (temp directory deleted while the DB was open).
- CI: clippy errors and rustfmt drift on `master`.

### Added
- Phase 3 core engine: project import with path validation, filesystem abstraction trait, metadata model, event bus, migration session state machine, checkpoint/rollback, SQLite-backed job persistence (`JobDb`)
- Integration tests: full pipeline import/session/checkpoint/resume, job persistence round-trip, event bus, metadata roundtrip, session-job store consistency (27 tests total)
- Phase 4 analyzer engine: adapter-based detectors behind `IAnalyzer` (`JarInspector`, `SourceScanner`, `BuildSystemDetector`, `DependencyGraphBuilder`); JAR inspection covers Fabric `fabric.mod.json`, Forge/NeoForge `mods.toml` (flat and `[[mods]]` array formats), legacy `mcmod.info`, and `META-INF/MANIFEST.MF` fallback with `Mod-ID` header; source scan detects Java/Kotlin/Scala; build system detects Gradle/Maven; Windows zip path backslash normalization; 8 unit tests + integration test over 9-fixture mod corpus

## [0.1.0] - 2026-11-18

### Added
- Initial release placeholder