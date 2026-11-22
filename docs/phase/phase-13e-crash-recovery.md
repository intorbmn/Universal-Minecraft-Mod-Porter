# Phase 13.E — Crash recovery

## Delivered in this increment

- Checkpoint snapshots flush copied file contents before publishing the manifest.
- `checkpoints.json` is written through a unique temporary file, `sync_all`, atomic rename, and Unix directory `fsync`.
- The manifest is persisted by `CheckpointStore::snapshot` before `MigrationEngine` begins applying edits. A process interruption therefore leaves a discoverable pre-transform checkpoint.
- Normal checkpoint-store saves use the same atomic persistence path.
- Added `ummp recover <project> --session <UUID>` to restore the latest checkpoint associated with a migration session. `ummp migrate --apply` prints the session UUID before waiting for completion.
- Added tests for manifest durability, recovery after partial file changes, and rejection of a truncated manifest.

## Recovery contract and limitation

Recovery currently supports **rollback to the pre-transform snapshot only**. Continuing is intentionally not offered: UMMP does not persist a deterministic migration edit plan, so replaying after a crash could apply a different edit set. Recovery is explicit by session UUID; automatic stale-process detection is not yet implemented.

The snapshot excludes the existing `SKIP_DIRS` (`.git`, build output, etc.), matching the established rollback contract. This is not a full-disk transaction and cannot restore files intentionally excluded from snapshots.

## Validation

CI for this branch is the validation gate. Do not mark Phase 13.E complete until build, tests, Clippy, formatting, desktop checks, and security audit all pass on the final commit. The CLI recovery path still needs a real process-kill smoke test on a disposable project.

## Usage

After an interrupted `ummp migrate --apply`:

```sh
ummp recover ./path/to/mod --session <session-uuid>
```

Add `--snapshots <directory>` if the checkpoint directory was customized.
