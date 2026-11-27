# Extending UMMP: rules, adapters and plugins

There are three ways to extend UMMP. Pick the cheapest one that does the job.

| Way | You write | Needs a rebuild of UMMP | Use it for |
|---|---|---|---|
| **Data rules** (§2) | TOML rows | yes, until the community KB ships (Phase 14.C) | detect a technology, add a loader template, rename loader APIs |
| **In-tree adapter** (§3) | Rust, `TechAdapter` | yes | a transformation that is part of UMMP itself |
| **Out-of-process plugin** (§4) | any executable that speaks JSON Lines; Rust has an SDK | **no** | your own checks and edits, installed per user |

Words used below: an **adapter** converts one technology or loader API to another; a **plugin**
is an executable UMMP starts for you; a **recipe** is a reusable rule set (Phase 14.D).

## 1. Security model (read this first)

- Plugins are **untrusted executables and are not sandboxed by the operating system**. A
  malicious plugin runs with your user's permissions. Install only plugins whose source you have
  read or whose author you trust. ADR 0019 explains the boundary.
- What UMMP does enforce: the plugin gets a scrubbed environment (no API keys or tokens), a
  30-second limit per call, an 8 MiB output cap, and secret-looking files are never sent to it.
- A plugin cannot write to your project. It returns proposed edits. UMMP checks that every path
  exists in the request, stays inside the project root and is not a `.jar` or `.class`, that
  `original` equals the file on disk, and only then applies the edit through `MigrationEngine`
  after a checkpoint. A stale or unsafe edit is rejected.
- Treat every input (JAR, repository, plugin output) as untrusted. Use `resolve_within` for any
  path that comes from input.

## 2. Data rules (no code)

**Detect a new technology.** Add a `[[rule]]` to `core/src/adapter_rules.toml`:

```toml
[[rule]]
tech = "my-tech"            # adapter id
level = "detect"            # detect | report | migrate
handling = "manual"         # auto | assisted | manual
message = "What the user must know or do"
files = ["ext"]             # file extensions, and/or
names = ["exact-name.cfg"]  # exact file names
any = ["pattern"]           # optional: one of these must occur in the file
```

Add a fixture line to `rules_detect_each_technology` in `core/tests/adapters.rs`.

**Support a new loader and Minecraft pair for conversion.** Add a `[[template]]` to
`core/src/loader_templates.toml` with `loader`, `minecraft`, `gradle`, `settings_gradle`,
`gradle_properties`, `build_gradle`. Take versions from the official template for that version
(NeoForge MDK, Fabric example mod) and say where in the file header. Placeholders:
`@MOD_ID@ @VERSION@ @GROUP@ @USER_DEPS@ @ACCESS@`.

**Rename loader APIs.** Built in: `[[rename]]` rows in `core/src/loader_rules.toml`
(`pair = "forge->neoforge"`). Per project or KB: mappings with `loader = "<from>-><to>"`; the
planner adds them as a last stage. For Mixin member renames use mappings with
`from_api = "pkg.Owner#name"`.

## 3. In-tree adapter (Rust)

Implement `ummp_core::adapter::TechAdapter`:

| Method | Use |
|---|---|
| `id`, `level` | name and how far it goes |
| `findings(cx)` | what was found, with file and line |
| `owns(cx, rel)` | files you rewrite yourself (generic handlers skip them) |
| `rewrite(cx, rel, text)` | change source text after the generic renames |
| `edits(cx)` | new or changed files (`FileEdit`, empty `original` = create) and `deletes` |

Register it in `AdapterRegistry::default()`. Read files only through `AdapterContext::text`
(path-checked, size-capped). Never write; return edits. Tests use real file formats and
synthetic names. `ummp adapters` prints the live list.

## 4. Out-of-process plugin

### 4.1 Tutorial: build, test and install the template

The template in `templates/plugin/` renames the synthetic type `DemoWidget` to `DemoGadget`.

1. Copy `templates/plugin/` to a new folder and edit `Cargo.toml`: change the package name and
   replace the `ummp-plugin-sdk` path with a version (the published SDK version).
2. Change `name` in `ummp-plugin.toml` and `Renamer::id` in `src/main.rs`. They must be equal,
   or `ummp plugins check` fails with "hello id does not match manifest name".
3. Edit `findings` and `edits` in `src/main.rs`. Both receive the project files UMMP chose to
   send (UTF-8 text up to 2 MiB each). Return an `Edit` with the whole old text as `original`.
4. Test it: `cargo test` runs your unit tests. To exercise the real protocol, build it and pipe
   a request: `echo '{"method":"hello"}' | target/debug/demo-renamer`.
5. Install: create `<data dir>/plugins/<name>/`, copy the executable and `ummp-plugin.toml`
   there, then run `ummp plugins check <that folder>`. `ummp plugins list` shows installed
   plugins.

In the UMMP repository CI builds the template and checks it end to end
(`core/tests/plugin_template.rs`), so it works as written.

### 4.2 The SDK

`ummp-plugin-sdk` exposes `PluginAdapter` (implement `id`, and `findings` and/or `edits`),
the request and response types, `Edit::replace_all`, `handle` (one request line in, one response
line out; handy in tests) and `run` (the stdin/stdout main loop). It does not depend on the UMMP
engine.

### 4.3 Protocol reference

`docs/PLUGIN_PROTOCOL.md` is the wire reference: manifest fields, framing, the `hello`,
`findings` and `edits` methods, limits. The golden files in
`tests/fixtures/golden/plugin-protocol/` are the executable form of that document; both the SDK
and the core are tested against them (ADR 0022). A wire change needs a new `ummp_api` number.

### 4.4 Rules for plugin authors

- One request per process start. Read one line, write one line, exit with status 0.
- Never print anything else to stdout. Use stderr for logs; UMMP does not parse it.
- Do not read the environment for secrets and do not contact the network from a migration
  plugin. Users will read your source before they trust it.
- Return edits for the files you were sent only. Anything else is rejected.
