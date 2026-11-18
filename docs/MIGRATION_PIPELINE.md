# MIGRATION_PIPELINE — Migration Workflow

## Pipeline steps
1. **Import** — open project or fetch from repo/archive. Validate path, safety, size limits.
2. **Detect** — loader, Minecraft version, Java version, dependencies, build system, languages.
3. **Analyze** — structure, APIs used, registries, events, mixins, resources, native code.
4. **Plan** — build dependency graph of changes; produce migration recipe + estimate.
5. **Transform** — source, build config, resources, datapacks, mixins, bytecode.
6. **Build** — run toolchain in sandbox; capture logs.
7. **Test** — unit/integration + game launch in sandbox; capture logs/crashes.
8. **Review** — diff viewer, AI review, human approval gate.
9. **Export** — produce migrated artifact + report + rollback snapshot.

## Step contract (each step)
- Clear input/output.
- Execution state persisted.
- Structured log.
- Timeout + retry policy.
- Error handling that does not corrupt prior steps.
- Checkpoint after completion; resumable on failure.

## Dependency graph
Steps run in order; `Plan` depends on `Analyze`; `Transform` depends on `Plan`; `Build` on `Transform`; `Test` on `Build`. Steps may be re-run independently (e.g. re-test after a fix) without re-importing.

## State machine
`PENDING → RUNNING → SUCCESS / FAILED / SKIPPED`. Failed steps are retryable; the session retains all prior results.

## User visibility
The UI/CLI shows current step, progress, logs, and a clear error with suggested next action.