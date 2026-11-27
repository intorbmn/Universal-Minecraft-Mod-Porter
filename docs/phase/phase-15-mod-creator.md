# Phase 15 — Mod Creator (planned, not started)

Requested by the user on 2026-11-27. Start only after Phases 13 and 14 are COMPLETED and the
user says so. This file records the request; it is not an approved design. Design it with
brainstorming and an ADR before any code.

## What the user asked for
- **Create a simple mod by picking features.** The user chooses features from a list; UMMP
  generates the project and packs it into a `.jar`.
- **Feature templates.** Each feature (examples to confirm: block, item, recipe, command)
  inserts its code into the project automatically.
- **AI option.** Instead of a template, a feature can be written by AI through the existing
  providers (`providers/`). AI output stays `FileEdit`s applied by `MigrationEngine` after a
  checkpoint (ADR 0005/0006).
- **Edit files directly.** A place in the app to write code file by file.
- **Build in the app.** Build the generated or edited source to a `.jar` (reuse the Phase 8
  build engine and malware scan).
- **Choose versions.** Pick the Minecraft version(s) and loader the mod targets; the goal is
  every version that can be modded.
- **Desktop app first.** The tool is delivered as the desktop app.
- **Bundled toolchain.** Software needed to build (JDK, and anything else) is installed with
  the app, so users do not set it up by hand.

## Decided by the user (2026-11-27)
- **Loaders:** Forge, NeoForge and Fabric.
- **JDK at install:** the installer shows a step asking whether to install a JDK; the user
  may skip it.
- **JDK manager in Settings:** after install, Settings lists installed JDK versions and lets
  the user add more or remove some, like NI Package Manager for LabVIEW.
- **No Free/Pro split for now.** All Mod Creator features are in the open-source app.

## Points to settle in the design
- "Every moddable version" is very large (each loader × version needs its own templates and
  APIs). Decide the first set and how templates are generated, tested and kept current.
- JDK manager details: which JDK versions (Minecraft versions need different Java versions),
  installer size, license of the bundled JDK (for example an OpenJDK build such as
  Temurin), and update path. Gradle still downloads Minecraft and loader libraries on the
  first build, so a network connection is needed at least once.
- Editor: plain text editor vs a code editor component; syntax checks.
- Templates must use synthetic sample names in repo fixtures (CLAUDE.md) and must not
  contain Mojang code.
- Mods users create are their own; UMMP does not publish them.
