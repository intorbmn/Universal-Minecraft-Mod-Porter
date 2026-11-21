# Performance Benchmarks

Phase 13A uses Criterion benchmarks in `core/benches/phase13.rs`.

## Workloads

- `analyze/one_jar`: inspect one deterministic fixture JAR.
- `analyze/300_jars`: analyze a generated 300-JAR corpus.
- `scan/300_jars`: scan the same 300-JAR workload.
- `pack/inspect_300_mods`: import and analyze a folder containing 300 JARs.
- `scan/5_mib`: scan a generated JAR containing a 5 MiB source entry.

The corpus is generated from checked-in fixture JARs at benchmark setup time. Fixture creation and copying are outside the timed sections. No network access or generated binary corpus is required.

## Running

From the repository root:

```sh
cargo bench -p ummp-core --bench phase13
```

Criterion output is written under `target/criterion`. Do not compare measurements across machines without recording the environment.

## Baseline

<escape>**Pending.**</escape> The GitHub-only editing environment used for this implementation does not provide a local Rust toolchain or executable checkout, so benchmark timings have not been run. No performance numbers or targets are claimed.

Record the following when the benchmark is run on a development machine:

- CPU model and physical/logical core count
- RAM
- OS and kernel
- `rustc --version`
- benchmark command
- commit SHA and date
- Criterion estimates for every workload

## Deferred guide workloads

The Phase 13 guide also requests plan + dry-run port of `tests/fixtures/forge-src-1.16` and remapping a 5 MiB JAR. Those are not represented by the current benchmark targets yet; they require dedicated deterministic setup and must be added before Phase 13A is considered complete. The 5 MiB scan workload is not a substitute for the remap workload.
