# KNOWN_LIMITATIONS

Limitations of the current code, so nobody mistakes "it ran" for "it is correct".
Each entry names the module and the phase expected to address it.

## Analyzer (Phase 4)
- **Minecraft version fallback.** When a descriptor has no readable Minecraft version, the
  analyzer reports `1.20.1` instead of "unknown". `ModMetadata.minecraft` is not optional yet,
  so a missing version cannot be told apart from a real 1.20.1.
- **Source projects.** Mods are detected from JARs only. A source tree's
  `src/main/resources/META-INF/mods.toml` or `fabric.mod.json` is not read, so `ummp analyze`
  on a source project reports `Mods: 0`.
- **Only the first mod** in a multi-mod `mods.toml` is reported.
- `tests/fixtures/generate.ps1` does not yet generate the three fixtures added in Phase 4
  gap closure (`fabric-classes`, `neoforge-b`, `quilt-a`); `generate.py` does.

## Knowledge base (Phase 5)
- **Seed data is illustrative, not authoritative.** `seed_knowledge()` contains sample rows
  (for example the `Block -> Blocks` rename and a "Fabric 1.12.2" row, which is really the
  separate Legacy Fabric project). It exists for tests. The CLI never seeds a user's KB.
- **Rows that fail to decode are skipped silently.** Query methods use
  `.filter_map(Result::ok)`; a corrupt row disappears instead of producing an error.
- No real mapping dataset (MCP, Yarn, Mojang, Parchment) is imported yet.

## Migration engine (Phase 6)
- **Only API renames in Java/Kotlin sources** are transformed. Not yet implemented: build
  script migration (Gradle plugin/loader versions, Java toolchain), descriptor migration
  (`mods.toml`, `fabric.mod.json`), resources/datapacks, Mixins, bytecode, and loader-to-loader
  migration.
- **No type resolution** (ADR 0007). A simple-name rule such as `Block -> X` renames every
  standalone `Block` token, whichever class it refers to. Member accesses (`foo.Block`) are
  left alone except inside `import` statements. Prefer qualified mappings.
- **Not rewritten:** string literals, comments, Kotlin string templates (`"${Block.ID}"`),
  and wildcard imports (`import net.minecraft.block.*`).
- **Versions must be numeric** (`1.20.1`). Snapshots and pre-releases (`24w14a`,
  `1.21-pre1`) are rejected by the planner.
- **Recipes are listed, not executed.** The report names applicable recipes, but recipe
  steps are not interpreted yet.
- Files over 2 MiB or not valid UTF-8 are skipped with a warning.

## Checkpoints (Phase 3)
- Snapshots are full copies of the project (minus `.git`, `.gradle`, `build`, `target`,
  `out`, `run`), so disk use grows with each `migrate --apply`. Old snapshots are never
  pruned automatically.
- Symlinks inside the project are neither snapshotted nor restored.
- `restore` does not touch the skipped directories. Build output produced after a
  checkpoint survives a rollback.

## Not started
Phases 7-14 (AI agents, build & test engine, advanced adapters, modpacks, UI, REST/MCP API,
production readiness, community) have no code yet.
