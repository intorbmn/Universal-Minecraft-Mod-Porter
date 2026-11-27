# Phase 15 — Mod Creator: detailed guide

Read [README.md](README.md) first (workflow, environment, GitHub quirks, definition of done).
The request and the user's decisions are in
[`../phase/phase-15-mod-creator.md`](../phase/phase-15-mod-creator.md).

**Do not start** until Phases 13 and 14 are COMPLETED and the user says so.

## 0. Skills
Required: `superpowers:brainstorming` then `superpowers:writing-plans`,
`andrej-karpathy-skills:karpathy-guidelines`, `superpowers:test-driven-development`,
`rust-cli-skills:rust-errors` / `rust-idiomatic` / `rust-testing`,
`rust-cli-skills:rust-filesystem` (project generation), `rust-cli-skills:rust-processes`
(JDK and Gradle), `rust-cli-skills:rust-design` (template trait),
`frontend-design:frontend-design` and `ui-ux-pro-max:ui-ux-pro-max` (new screens),
`claude-api:claude-api` (AI features), `ecc:architecture-decision-records`,
`claude-security:claude-security` (downloads, user code builds).
If one is missing, ask the user to install it (template in `phase-11-ui.md` §0.2).

## 1. Goal
A user with no setup creates a simple mod in the desktop app:
1. Pick loader (**Forge, NeoForge, Fabric**) and Minecraft version.
2. Pick features from a list; each inserts template code, or is written by AI.
3. Edit any file in the app.
4. Build to a `.jar` in the app with a JDK the app manages.

## 2. Decided by the user (2026-11-27) — do not reopen
- Loaders: Forge, NeoForge, Fabric. Target: every version those loaders support; the first
  milestone may ship a smaller set, but the design must scale to all of them.
- The installer shows a step asking whether to install a JDK. Skipping is allowed.
- Settings has a **JDK manager**: list installed JDKs, add a version, remove a version
  (like NI Package Manager for LabVIEW).
- Desktop app is the main surface.
- No Free/Pro split for now: everything is in this repo, Apache-2.0.

## 3. Depends on (reuse, do not rebuild)
| Need | Use |
|---|---|
| Build to `.jar`, timeouts, cancel | Phase 8 build engine (`core/src/build/`) |
| Scan the built `.jar` | Phase 8 malware scan |
| Loader and version data | Phase 9 adapters and loader conversion |
| AI code | `providers/` + Phase 7 agents; output only `FileEdit`s through `MigrationEngine` after a checkpoint (ADR 0005/0006) |
| Desktop screens, settings, jobs | Phase 11 (`ui/`, `ui/src/lib/api.ts`, `AppServices`) |
| Releases and installer | Phase 13.F (cargo-dist, Tauri bundles) |

## 4. Questions for the user (defaults in brackets)
1. First feature list: [block, item, recipe, creative tab, simple command].
2. First version set for the first PR: [the latest release of each loader, plus 1.20.1].
3. Where JDKs come from: [Eclipse Temurin via the Adoptium API, downloaded on demand,
   checksum verified]. Bundling JDKs inside the installer makes it hundreds of MB.
4. Where JDKs are installed: [the UMMP data directory, one folder per version].
5. Editor: [a code editor component with Java syntax highlighting; no full IDE].
6. Mod ID, name and package fields: [required, validated before generation].

## 5. Milestones
Each milestone is one PR with tests that can fail (README §4).

### 15.A — Template model and generator (core)
- ADR (next free number) for the template format: data files (manifest + code snippets per
  loader/version range), not code in Rust.
- `ModSpec { loader, mc_version, mod_id, name, package, features[] }` → generated project.
- Generation writes through `MigrationEngine` with a checkpoint, inside a new directory
  validated with `resolve_within`.
- Tests: generate an empty mod for each loader; golden file trees; invalid mod ID refused.

### 15.B — Feature templates
- Each feature declares the versions it supports, the files it adds and the snippets it
  inserts (registration code, assets, lang, recipes).
- Inserting two features into the same file must not break it; removing a feature removes
  its code.
- Tests: golden output per feature × loader; conflict cases.
- Templates use synthetic names in repo fixtures (CLAUDE.md) and contain no Mojang code.

### 15.C — JDK manager
- Core service: list, install, remove JDKs; map Minecraft version → required Java version
  (for example Java 8, 17, 21; confirm from loader docs).
- Download with checksum and signature check where available; resume or clean up on cancel.
- Desktop Settings screen: installed list, add, remove, disk use.
- Installer step: Tauri NSIS/MSI hook asking "Install JDK now?" that runs the same service.
  Check what Tauri 2 bundlers support before designing the hook.
- Tests: fake download server; checksum mismatch refused; remove deletes only its folder.

### 15.D — Build in the app
- Pick the right JDK for the mod's version, run Gradle through the Phase 8 engine, scan,
  show the `.jar` location.
- First build needs the network (Gradle downloads Minecraft and loader libraries). Say so
  in the UI.
- Tests: build job with a fake Gradle; cancel; missing JDK gives a clear message with a
  link to the JDK manager.

### 15.E — Mod Creator screen and editor
- Screens: new mod form → feature picker → file tree + editor → build.
- Saving an edit goes through `MigrationEngine` (checkpoint, rollback).
- Tests: Vitest for each screen, one Playwright flow with mocked backend.

### 15.F — AI features
- "Write this feature with AI": prompt from the spec and the chosen feature, provider
  picked on the screen (as on the AI tab), output previewed as a diff before applying.
- Tests: fake provider returning edits; invalid edit refused; nothing written without
  the user's approval.

### 15.G — Version coverage
- Grow the version set toward every version the three loaders support. Add a CI matrix
  that generates and compiles one sample mod per loader × version group (nightly or
  manual, not on every PR).

## 6. Docs (same PRs)
`PROJECT_STATUS.md`, this phase file, `CHANGELOG.md`, `KNOWN_LIMITATIONS.md`, user guide
section "Create a mod", and the JDK manager in the troubleshooting guide.

## 7. Risks
- **Scale:** every loader × version needs templates; without a CI matrix they rot.
- **Downloads:** JDKs and Gradle libraries are large; handle slow networks and cancel.
- **User code runs:** builds run the user's own Gradle files on their machine. Keep the
  Phase 8 rules (timeouts, scan) and say plainly that builds run with the user's rights.
- **Licenses:** JDK distribution terms; loader and mapping licenses for templates.

## 8. Definition of done
All milestones merged with CI green; a user on a clean Windows machine installs the app,
installs a JDK from the installer step or Settings, creates a mod with two features for
each of Forge, NeoForge and Fabric, builds it, and the `.jar` loads in that loader
(manual check, recorded with the versions used).
