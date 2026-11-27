# Contributing to Universal Minecraft Mod Porter

Thank you for helping. This page is written so that you can go from a clean clone to a
reviewed pull request without asking anyone. If a step does not work as written, that is a bug
in this file: open an issue.

By taking part you agree to the [Code of Conduct](CODE_OF_CONDUCT.md). Code is licensed under
Apache-2.0 (see [LICENSE](LICENSE)); contributing code means you license it the same way.

## 1. Set up

You need Git and a recent Rust toolchain from <https://rustup.rs>. The exact toolchain is pinned
in `rust-toolchain.toml`; rustup installs it the first time you run cargo.

```sh
git clone https://github.com/intorbmn/universal-minecraft-mod-porter.git
cd universal-minecraft-mod-porter
cargo build --all
```

If rustup cannot download the pinned toolchain (for example in a sandbox without internet access
to static.rust-lang.org), check that `rustc -V` under `RUSTUP_TOOLCHAIN=stable` prints the
pinned version and run every cargo command with that variable set. Never unpin the toolchain to
get around this.

The desktop app (`ui/`) is a separate workspace that needs Node.js and, on Linux, GTK/WebKit
libraries. You only need it if you change the UI.

## 2. The four commands

These must all pass before you open a pull request. CI runs the same four.

```sh
cargo build --all
cargo test --all
cargo clippy --all --all-targets -- -D warnings
cargo fmt --all -- --check
```

## 3. How we write code

- **Test first.** Write a test, run it, watch it fail for the right reason, then make it pass.
  A new behaviour without a test that failed first is not accepted.
- **Every test must be able to fail.** For key tests, break the code on purpose once and confirm
  the test goes red, then restore it. Note what you broke in the pull request.
- **Fixtures look like real input.** A JAR fixture contains real binary `.class` files; a
  descriptor uses the real schema of its loader. Generators live in `tests/fixtures/`.
- **Synthetic names only.** Sample data uses clearly made-up names such as
  `DemoWidget -> DemoGadget`. Never use names that look like real Minecraft or loader APIs in
  tests or fixtures. (Real names are fine in the community knowledge base, which is a separate
  repository, because that is its purpose.)
- **Untrusted input.** Treat every input JAR, repository and plugin as untrusted. Use
  `resolve_within` for any path that comes from input.
- **Edits go through the engine.** Project files are changed only by `MigrationEngine` after a
  checkpoint (ADR 0005, 0006). This includes AI output and plugin output.
- **Lints.** `unsafe` code is forbidden; clippy pedantic and nursery lints are on.
- **Layers.** Follow the modular monolith layering in `docs/SAD.md`. The desktop UI calls
  application services only (ADR 0004).

## 4. Decisions need an ADR

A decision that someone could reasonably have made differently gets an Architecture Decision
Record in `docs/decisions/`: copy the newest file, use the next number (check the folder; do not
reuse one), keep it to context, decision, consequences and rejected alternatives, and add a row
to `docs/decisions/README.md`.

## 5. Docs go in the same pull request

Update, as far as they apply: `docs/PROJECT_STATUS.md`, the phase file in `docs/phase/`,
`CHANGELOG.md` and `docs/KNOWN_LIMITATIONS.md`. A phase is COMPLETED only with test evidence
against its acceptance criteria.

## 6. Commits and pull requests

- Work on a branch, never directly on `master`.
- Use Conventional Commits: `feat:`, `fix:`, `docs:`, `test:`, `chore:`.
- Keep a pull request to one topic. The template asks for what changed, how you tested it and
  which test you broke on purpose.
- A maintainer reviews every pull request. Nothing is merged automatically.

## 7. Where things live

| Path | Contents |
|---|---|
| `core/` | engine library: analysis, migration, adapters, AI, build, plugins |
| `cli/` | the `ummp` command line |
| `providers/` | HTTP AI providers (the only crate that talks to model APIs) |
| `server/` | local REST API and MCP server |
| `plugin-sdk/` | `ummp-plugin-sdk`, the SDK for plugin authors |
| `templates/plugin/` | the template plugin, built and checked in CI |
| `ui/` | Tauri + Svelte desktop app |
| `tests/fixtures/` | fixture mods, golden outputs |
| `docs/` | architecture, ADRs, phase files, user guides |

## 8. Other ways to contribute

- **Rules, mappings, recipes:** open a "Rule contribution" issue. A separate knowledge-base
  repository for these is planned (Phase 14.C); until it exists, rules are reviewed here.
- **Plugins:** see `docs/PLUGIN_SDK.md`. Plugins are untrusted executables, so keep yours small
  and readable.
- **Bugs and port problems:** use the issue templates. Never attach mod source you do not have
  the right to share.
- **Security problems:** do not open a public issue; follow [SECURITY.md](SECURITY.md).
