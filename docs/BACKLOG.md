# BACKLOG — not done yet, to revisit

Items deferred from finished phases. Each line says what is missing and why it was left.
Move an item into a phase file when it is picked up.

## Phase 8 — Build & Testing Engine
- [ ] **Real GitHub run not yet verified.** Needs the private build repo
  `intorbmn/ummp-build-sandbox`, which this session could not create. Create it, then run
  `ummp build tests/fixtures/build-demo` and record the result in `docs/phase/phase-08-build.md`.
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
