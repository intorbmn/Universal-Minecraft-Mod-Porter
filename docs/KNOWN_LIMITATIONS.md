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
  (for example the synthetic `DemoWidget -> DemoGadget` rename and a "Fabric 1.12.2" row, which is really the
  separate Legacy Fabric project). It exists for tests. The CLI never seeds a user's KB.
- **Rows that fail to decode are skipped silently.** Query methods use
  `.filter_map(Result::ok)`; a corrupt row disappears instead of producing an error.
- No real mapping dataset (MCP, Yarn, Mojang, Parchment) is imported yet.

## Migration engine (Phase 6)
- **Transformed so far:** API renames in Java/Kotlin sources (6.A), and Minecraft/loader
  version requirements in `fabric.mod.json`, `quilt.mod.json` and `META-INF/[neoforge.]mods.toml`
  (6.B). Not yet: build scripts, resources/datapacks, Mixins, bytecode, loader-to-loader.
- **Descriptors:** the loader requirement is updated only when the KB has a compatibility row
  for the exact target version; otherwise it is left as is with a warning. `ummp migrate
  --mappings` uses a KB with no compatibility rows, so it never updates loader versions.
  `loaderVersion` (the javafml language provider) and `mods.toml` → `neoforge.mods.toml`
  renames (NeoForge 1.20.5+) are reported, not changed. Escaped JSON strings are skipped.
- **No type resolution** (ADR 0007). A simple-name rule such as `DemoWidget -> DemoGadget` renames every
  standalone `DemoWidget` token, whichever class it refers to. After a dot it is renamed only
  when the qualifier is all-lowercase (ADR 0008), so `x.DemoWidget` on a lowercase variable
  is renamed too. Prefer qualified mappings.
- **Not rewritten:** string literals, comments, Kotlin string templates (`"${DemoWidget.ID}"`),
  and wildcard imports (`import com.example.*`).
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
