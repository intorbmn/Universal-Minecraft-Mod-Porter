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

UMMP reads `ummp.toml` from the current working directory first. If it is absent, it checks
the user configuration directory:

- Windows: `%APPDATA%/ummp/ummp.toml`
- macOS: `~/Library/Application Support/ummp/ummp.toml`
- Linux: `$XDG_CONFIG_HOME/ummp/ummp.toml`, or `~/.config/ummp/ummp.toml`

Example:

```toml
output_dir = "D:/Minecraft/ported"
sandbox_repo = "your-name/ummp-build-sandbox"
provider = "anthropic"
```

Precedence is CLI flag > `UMMP_*` environment variable > config file > built-in default.
Supported environment variables are `UMMP_OUTPUT_DIR`, `UMMP_BUILD_REPO`, and
`UMMP_PROVIDER`. CLI overrides are `--output`, `--repo`, and `--provider` on the
commands that expose those options.

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

The JSON envelope is versioned with `schema: 1` and contains `command` and `result`.
At this point in Phase 12.A, the versioned envelope is implemented for `analyze --json`
and `scan --json`. The global `--json` contract and structured output for every command
remain open work; do not assume other commands are machine-readable yet.

Example:

```sh
ummp scan ./my-mod --json
```
