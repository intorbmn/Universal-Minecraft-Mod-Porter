# CLI contract

UMMP's CLI is intended for interactive use, scripts and CI. The contract is versioned so
external callers can detect incompatible output changes.

## Shell completions

Generate a completion script and install it using your shell's normal completion mechanism:

```sh
ummp completions bash
ummp completions zsh
ummp completions fish
ummp completions powershell
```

## Configuration

Configuration is loaded in this order:

1. `--config PATH`, when explicitly supplied (a missing or invalid file is an error).
2. `./ummp.toml` in the current working directory.
3. `<data_dir>/ummp.toml` in UMMP's shared per-user data directory.
4. Built-in defaults.

The shared data directory can be overridden with `UMMP_DATA_DIR`. Otherwise it is:

- Windows: `%APPDATA%/io.github.intorbmn.ummp`
- macOS: `~/Library/Application Support/io.github.intorbmn.ummp`
- Linux: `$XDG_DATA_HOME/io.github.intorbmn.ummp`, or `~/.local/share/io.github.intorbmn.ummp`

Example:

```toml
output_root = "D:/Minecraft/ported"
kb = "D:/Minecraft/mappings.db"
sandbox_repo = "your-name/ummp-build-sandbox"
ai_provider = "anthropic"
openai_base_url = "http://localhost:11434/v1"
```

Precedence is CLI flag > `UMMP_*` environment variable > TOML > built-in default.
Supported variables are `UMMP_OUTPUT_ROOT`, `UMMP_KB`, `UMMP_BUILD_REPO`,
`UMMP_AI_PROVIDER`, and `UMMP_OPENAI_BASE_URL`. The previous
`UMMP_OUTPUT_DIR` and `UMMP_PROVIDER` names remain accepted for compatibility.

API keys and other secrets are not valid TOML fields. Configure them with
`ummp ai-key set`, never in `ummp.toml`.

## Exit codes

| Code | Meaning |
|---:|---|
| 0 | Success |
| 1 | General error |
| 2 | Refused by a safety/risk gate |
| 3 | Completed with items left for the user |
| 4 | Build failed |
| 5 | Invalid CLI input |

Help and version output return 0. Invalid command-line syntax returns 5.

## JSON

Use `--json` globally, before or after the subcommand. The envelope is versioned with
`schema: 1`; human-readable command logs are sent to stderr.

```sh
ummp --json scan ./my-mod
ummp analyze ./my-mod --json
```

`analyze` and `scan` return their application `*View` result objects. `adapters`,
`remap-jar`, `port`, `build`, and `rollback` return structured command-specific results.
Long-running `port` and `build` operations run as `AppServices` jobs; their logs go to stderr
in JSON mode. Remaining commands are still being migrated and may return an envelope
containing only `exit_code`.

```json
{"schema":1,"error":{"kind":"invalid_input","message":"..."}}
```

Shell completion generation cannot be combined with `--json`.

## AI credentials

AI provider keys are shared by the desktop app and CLI through the interim credential
store. Enter a key through hidden terminal input, or pipe one line from stdin:

```sh
ummp ai-key set --provider anthropic
ummp ai-key set --provider openai --base-url http://localhost:11434/v1
printf '%s\n' "$MY_AI_KEY" | ummp ai-key set --provider openai
ummp ai-key status
ummp ai-key status --provider anthropic
ummp ai-key delete --provider openai
ummp ai-key delete
```

A key is never accepted as a command-line argument. Status shows only a masked value and
the credentials path. The full key is not printed. Environment variables
(`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`) override saved credentials and are not copied
into the store. The OpenAI-compatible base URL is saved with the credential and can be
overridden with `--base-url` for an AI command.

The file-backed store is temporary and weaker than an OS keyring: another process running
as the same user can read it. On Unix the file is created with mode `0600`; on Windows it
is kept in the per-user application data directory. Do not place it in a repository or
share it.


### Application-service routing

The `migrate` command uses the application plan view for dry runs and starts an
application migration job only when `--apply` is supplied. The `pack migrate` command
also runs as an application job. Both commands preserve command-level KB/mappings
options; `migrate --snapshots` selects the checkpoint directory used by the job.

JSON results for `migrate` contain the versioned `PlanView`; applied migrations include
the completed job result alongside the reviewed plan. `pack migrate` returns the pack
job result and retains exit code 2 for a High-risk refusal and 3 for manual items.
