# BACKLOG — not done yet, to revisit

Items deferred from finished phases. Each line says what is missing and why it was left.
Move an item into a phase file when it is picked up.

## Phase 9 — Advanced Technology Adapters
- [ ] Build templates for more Minecraft versions (only 1.21.1 today) and for Forge
  (Fabric → Forge converts descriptor/entry, but the build needs a ForgeGradle template).
- [ ] `quilt.mod.json` as a conversion **target** (Quilt is treated as Fabric).
- [ ] Automatic port of loader API semantics (events, registries, networking, config): today
  each use is listed as `assisted`; a KB of `<from>-><to>` mappings would cover common ones.
- [ ] Dependency id mapping between loaders (KB `kind = "dependency"`), e.g. API libraries.
- [ ] Rendering/shader/networking adapters beyond detection (need version-diff data).
- [ ] Native code build (C/C++/Rust) and JNI ABI checks; JAR-only native libraries.
- [ ] Kotlin entry classes: the bridge is Java and calls the Kotlin class; `ModInitializer`
  stripping is Java-only.
- [ ] Mixin refmap handling when mapping namespaces differ (intermediary/yarn ↔ Mojang).
- [ ] Access widener fields from AT (AT has no field descriptor; left as commented lines).
- [ ] `port` from a Git URL (today: folder or zip).
- [x] Demo `gradle-wrapper.jar` validated by `gradle/actions/wrapper-validation` on GitHub.
- [ ] Validate `gradle-wrapper.jar` checksums locally too (today: Medium finding, `--yes`).

## Phase 8 — Build & Testing Engine
- [x] Real GitHub run verified (2026-11-19, 3/3).
- [ ] Delete old `ummp/*` branches in the build repo automatically (they pile up today).
- [ ] Per-stage logs when artifact storage is blocked (fallback gives stage results only).
- [ ] Minecraft **client** launch check (needs a display/GPU; e.g. Xvfb on Linux runners).
- [ ] **Maven** builds (only Gradle is supported).
- [ ] **Native** builds (C/C++/Rust parts of a mod; JNI).
- [ ] **Docker/VM isolation** for `--local` builds (today: copy folder, scrubbed env, timeout).
- [ ] Download a missing **JDK** automatically (today: error with install hint).
- [ ] Build **JAR-only** mods by dropping them into a real server (today: `scan` only).
- [ ] Run a mod's own **in-game integration tests** (GameTest).
- [ ] **Dynamic/behavioral** malware analysis (today: static rules only).
- [ ] Scan rules: compare native libraries against what the mod declares; signature
  database of known malware families.
- [ ] Windows process-tree kill (`taskkill /T`) is implemented but only tested on Linux CI.
- [ ] Proxy support for local builds: the env allow-list drops `HTTPS_PROXY`/`JAVA_TOOL_OPTIONS`.

## Phase 7 — AI agents
- [ ] Streaming responses; live API test (no key in CI).
- [ ] Agent memory across sessions.

## Phase 6 — Migration engine
- [ ] Real mapping dataset in the knowledge base (risks R1/R4).
- [ ] Method/field renames in bytecode and Mixin injector targets (member mappings).
