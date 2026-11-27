## What and why

<!-- One or two sentences. Link the issue if there is one. -->

## How I tested it

- [ ] `cargo build --all`
- [ ] `cargo test --all`
- [ ] `cargo clippy --all --all-targets -- -D warnings`
- [ ] `cargo fmt --all -- --check`

New or changed behaviour has a test that I watched fail first.

**Test I broke on purpose, and what I broke:**

<!-- Example: "changed TO in the template; plugin_template.rs went red; restored" -->

## Checklist

- [ ] Fixtures use real formats and synthetic names (no real Minecraft API names)
- [ ] Paths that come from input go through `resolve_within`
- [ ] Project edits go through `MigrationEngine` (no direct writes)
- [ ] A decision that could go another way has an ADR in `docs/decisions/`
- [ ] Updated `docs/PROJECT_STATUS.md`, the phase file, `CHANGELOG.md`, `docs/KNOWN_LIMITATIONS.md` where they apply
- [ ] User-facing text is clear to someone who has not read the code
