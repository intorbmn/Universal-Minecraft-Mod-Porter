# Phase 4 — Analyzer Engine

| Field | Value |
|---|---|
| Phase ID | P4 |
| Objective | Detect loader, Minecraft version, Java version, dependencies, build system, languages; produce analysis report + dependency graph. |
| Scope | JAR analyzer, source analyzer, metadata/version/loader/Java detectors, dependency analyzer, build system analyzer, technology detector. |
| Prerequisites | Phase 3. |
| Modules | ENG-01. |
| Architecture | Adapter-based detectors behind `IAnalyzer`. |
| Tasks | 4.1 JAR inspection (manifest, mods.tomper, fabric.mod.json) 4.2 Source tree scan 4.3 Version/loader/Java detectors 4.4 Dependency graph builder 4.5 Build system detection (gradle/maven) 4.6 Analysis report + export 4.7 Analyzer test corpus fixtures |
| Dependencies | Phase 3. |
| Deliverables | Project Analysis Report, Dependency Graph, Technology Compatibility Matrix, test corpus. |
| Tests | Unit per detector; integration on fixture mods (Forge, Fabric, Quilt, Kotlin, native). |
| Risks | Obfuscated JARs; ambiguous loader detection. |
| Acceptance Criteria | 10+ fixture mods analyzed correctly; report is machine-readable and human-readable. |
| Complexity | M |
| Status | COMPLETED |
| Notes | Adapter-based detectors behind `IAnalyzer`: `JarInspector`, `SourceScanner`, `BuildSystemDetector`, `DependencyGraphBuilder`. JAR inspection covers Fabric (`fabric.mod.json`), Forge/NeoForge (`mods.toml`), legacy `mcmod.info`, and `META-INF/MANIFEST.MF` fallback. Source scan detects Java/Kotlin/Scala. Build system detects Gradle/Maven. 5 unit tests pass. |