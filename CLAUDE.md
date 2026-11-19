# CLAUDE.md — working on UMMP

Universal Minecraft Mod Porter: a Rust modular monolith (`core/` library + `cli/` binary)
that analyzes Minecraft mods and migrates them across versions and loaders. The product and
process requirements are in `universal mod porter.txt` (the master prompt). The architecture
is in `docs/SAD.md` and `docs/decisions/`.

## Before any change
1. Read `docs/PROJECT_STATUS.md`, the current phase file in `docs/phase/`, and
   `docs/KNOWN_LIMITATIONS.md`.
2. Load the skills listed for the phase you are working on (table below). Loading the
   skills is part of starting the milestone, not optional.
3. Work on a branch, never directly on `master`.

## Commands (must all pass before a commit, same as CI)
```sh
cargo build --all
cargo test --all
cargo clippy --all --all-targets -- -D warnings
cargo fmt --all -- --check
```
The toolchain is pinned in `rust-toolchain.toml`. If rustup cannot download the pinned
version (for example in a sandbox without access to static.rust-lang.org), check that
`rustc -V` under `RUSTUP_TOOLCHAIN=stable` prints the pinned version, then run the commands
with that variable set. Never unpin the toolchain to get around this.

## Rules that are easy to break
- **A status is COMPLETED only with test evidence** against the phase's acceptance criteria.
  Writing code without running the tests is not enough. Earlier phases were wrongly marked
  done; see the notes in `docs/phase/`.
- **Every test must be able to fail.** For key tests, break the code on purpose once and
  confirm the test goes red. Fixtures must look like real inputs: JARs contain binary
  `.class` files, and descriptors use real schemas.
- **Do not move on to the next phase on your own** (master prompt §14). Finish the current
  milestone, report it, and wait for the user to review.
- Every significant decision gets an ADR in `docs/decisions/` (next number: 0010).
- Update `PROJECT_STATUS.md`, the phase file, `CHANGELOG.md`, and `KNOWN_LIMITATIONS.md`
  in the same change as the code.
- Edits are applied only by `MigrationEngine` after a checkpoint (ADR 0005/0006). This
  includes AI output from Phase 7 onwards.
- Treat every input JAR, repository, and plugin as untrusted. Use `resolve_within` for any
  path that comes from input.
- Sample data uses clearly synthetic names (`DemoWidget -> DemoGadget`). Never use names
  that look like real Minecraft APIs.

## Skills per phase
"Always" skills apply in every phase. Where several skills cover the same job, use the one
listed here and not the duplicate (for example `ecc:tdd-workflow`, `ecc:rust-test` or
`rust-cli-skills:rust-build`).

| When | Skills |
|---|---|
| **Always: design a milestone** | `superpowers:brainstorming`, then `superpowers:writing-plans`; `andrej-karpathy-skills:karpathy-guidelines` (avoid overengineering, master prompt §12) |
| **Always: implement** | `superpowers:test-driven-development`; `rust-cli-skills:rust-errors`, `rust-cli-skills:rust-idiomatic`, `rust-cli-skills:rust-testing` |
| **Always: debug** | `superpowers:systematic-debugging`; `ecc:rust-build` for compiler or borrow-checker errors |
| **Always: finish** | `superpowers:verification-before-completion`; `ecc:rust-review`; `pr-review-toolkit:review-pr`; `commit-commands:commit-push-pr` |
| **Always: decisions** | `ecc:architecture-decision-records` |
| 6 Migration Engine | `rust-cli-skills:rust-filesystem`, `rust-cli-skills:rust-io` (descriptor, build-script, and resource transforms) |
| 7 AI Agent System | `claude-api:claude-api` (provider abstraction, tool calling, structured output, cost); `superpowers:brainstorming` first. AI must return `FileEdit`s only (ADR 0005) |
| 8 Build & Testing Engine | `rust-cli-skills:rust-processes`, `rust-cli-skills:rust-concurrency` (Gradle/Java runner, timeouts, cancellation); `claude-security:claude-security` before running untrusted builds |
| 9 Advanced Adapters | `rust-cli-skills:rust-design` (adapter traits); `superpowers:brainstorming` for each new adapter family |
| 10 Modpack Migration | `rust-cli-skills:rust-concurrency` (parallel analysis); `rust-cli-skills:rust-performance` |
| 11 User Interface (Tauri) | `frontend-design:frontend-design`, `ui-ux-pro-max:ui-ux-pro-max`. The UI calls application services only (ADR 0004) |
| 12 CLI, API & Integration | `rust-cli-skills:rust-cli-interface`; `example-skills:mcp-builder`, `ecc:mcp-server-patterns` (MCP server); `ecc:api-design` (REST) |
| 13 Production Readiness | `claude-security:claude-security` (full audit); `rust-cli-skills:rust-performance`, `rust-cli-skills:rust-memory`; `rust-cli-skills:rust-distribution` (installer, releases) |
| 14 Community & Ecosystem | `example-skills:doc-coauthoring` (contributor and plugin-SDK docs); `pr-review-toolkit:review-pr` for community recipes |

Skill names come from the plugins installed on the maintainer's account. If a skill is
not available in a session, say so and continue with the same practice by hand. Do not
skip the practice.

## Layout
| Path | Contents |
|---|---|
| `core/src/` | `project`, `filesystem`, `metadata`, `session`, `events`, `checkpoint`, `jobs`/`persistence`, `analyzer`, `knowledge`, `migration`, `descriptor`, `build_script`, `resources`, `mixin`, `bytecode` |
| `core/tests/` | Integration tests (`integration.rs` for Phase 3, `migration.rs` and `bytecode_jvm.rs` for Phase 6) |
| `cli/src/main.rs` | The `ummp analyze`, `migrate`, `remap-jar`, and `rollback` commands |
| `tests/fixtures/` | Mod corpus (`generate.py`), the `forge-src-1.16` project, and `golden/` expected outputs |
| `docs/` | PRD, SAD, ROADMAP, phase files, ADRs, KNOWN_LIMITATIONS, PROJECT_STATUS |
