# ADR 0022: Plugin SDK crate and golden-file protocol contract

**Status:** Accepted
**Date:** 2026-11-27
**Scope:** `ummp-plugin-sdk`, `templates/plugin`, `docs/PLUGIN_PROTOCOL.md`, `core/src/plugins.rs`

## Context

ADR 0019 defined the out-of-process plugin protocol (API 1) but left plugin authors to write
the JSON framing themselves. Phase 14.B needs an SDK so that a plugin is a few dozen lines,
plus a template that CI proves still works. The SDK types and the host types in
`core/src/plugins.rs` must not drift apart, because a drift would only show up when a user
runs a third-party plugin.

Two ways to share the protocol types were considered:

1. One definition in `ummp-plugin-sdk`, imported by `ummp-core`.
2. Two independent definitions that are both tested against the same golden JSON files.

## Decision

Use option 2. `ummp-plugin-sdk` is a small crate (serde and serde_json only) that does not
depend on `ummp-core`. The files in `tests/fixtures/golden/plugin-protocol/` are the contract.
The SDK suite parses the golden requests and compares its responses to the golden responses.
The core suite builds its requests through `hello_request`, `findings_request` and
`edits_request` and compares them to the same files, and parses the golden responses with its
own types. Changing a field name on either side turns one of the two suites red.

The template plugin lives in `templates/plugin/` as a workspace member.
`core/tests/plugin_template.rs` builds it with cargo, runs `PluginManager::check`, applies its
edit through `MigrationEngine` after a checkpoint, and rolls the checkpoint back.

## Consequences

- Plugin authors depend on a crate with no UMMP engine code, so the SDK stays small and can be
  published separately.
- The core keeps its freedom to change internal types as long as the wire format and the golden
  files agree. A wire change is a protocol change and needs a new `ummp_api` number.
- Two definitions mean two places to edit when the protocol grows. The golden files make a
  forgotten edit fail in CI instead of in the field.
- The SDK does not add sandboxing. Plugins remain untrusted executables (ADR 0019).

## Alternatives rejected

- Core depends on the SDK (option 1): rejected by the maintainer in favour of keeping the core
  free of a dependency that third parties version independently.
- Generating types from a JSON Schema: deferred; the protocol has three methods and the golden
  files are cheaper to maintain than a generator.
