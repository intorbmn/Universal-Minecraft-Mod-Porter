# Phase 13.H — Bilingual user documentation

Added:
- docs/USER_GUIDE.md (English)
- docs/USER_GUIDE.vi.md (Vietnamese)

Both guides cover installation, MCP registration, source porting, modpack migration, report review, build backends, AI assistance, rollback/recovery, update checks, Java/Gradle/Maven 429 troubleshooting, risk refusals and known limitations.

Acceptance:
- [ ] Verify every CLI example against current `--help` output on a built RC binary.
- [x] Local Markdown links in README and both user guides resolve against the repository tree.
- [x] CLI examples were cross-checked against the current Clap argument definitions; executing a built RC binary's `--help` remains pending.
- [ ] Confirm Windows/Linux/macOS installer instructions match the actual RC artifacts.
- [x] Stable CI passed on commit `209933d4399f456c09efc9fd79399ee3e1dd67c8`: run [#971](https://github.com/intorbmn/Universal-Minecraft-Mod-Porter/actions/runs/45117075173) (build/test/lint, desktop, security-audit, distribution-plan).
- [x] cargo-dist release plan passed: run [#67](https://github.com/intorbmn/Universal-Minecraft-Mod-Porter/actions/runs/45117081636).
