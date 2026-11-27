# Template plugin: demo-renamer

A minimal UMMP plugin written with `ummp-plugin-sdk`. It renames the synthetic type
`DemoWidget` to `DemoGadget` and reports each file that contains it.

```sh
cargo test                      # unit tests
cargo build                     # builds target/debug/demo-renamer
echo '{"method":"hello"}' | target/debug/demo-renamer
```

Install by copying the executable and `ummp-plugin.toml` into
`<UMMP data dir>/plugins/demo-renamer/` and running `ummp plugins check` on that folder.

Plugins are untrusted executables and are not sandboxed by the operating system. See
`docs/PLUGIN_SDK.md` in the UMMP repository for the tutorial, the security model and the
protocol reference.
