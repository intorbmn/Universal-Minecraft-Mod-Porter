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

`analyze` and `scan` return their application `*View` result objects. Other commands
currently return an envelope containing the process `exit_code` while their detailed
result views are being migrated. Errors use:

```json
{"schema":1,"error":{"kind":"invalid_input","message":"..."}}
```

Shell completion generation cannot be combined with `--json`.
