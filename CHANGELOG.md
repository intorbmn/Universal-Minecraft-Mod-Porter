# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Phase 3 core engine: project import with path validation, filesystem abstraction trait, metadata model, event bus, migration session state machine, checkpoint/rollback, SQLite-backed job persistence (`JobDb`)
- Integration tests: full pipeline import/session/checkpoint/resume, job persistence round-trip, event bus, metadata roundtrip, session-job store consistency (27 tests total)
- Phase 4 analyzer engine: adapter-based detectors behind `IAnalyzer` (`JarInspector`, `SourceScanner`, `BuildSystemDetector`, `DependencyGraphBuilder`); JAR inspection covers Fabric `fabric.mod.json`, Forge/NeoForge `mods.toml`, legacy `mcmod.info`, and `META-INF/MANIFEST.MF` fallback; source scan detects Java/Kotlin/Scala; build system detects Gradle/Maven; 5 unit tests

## [0.1.0] - 2026-11-18

### Added
- Initial release placeholder