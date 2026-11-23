# Performance Benchmarks

Phase 13A uses Criterion benchmarks in `core/benches/phase13.rs`.

## Workloads

- `analyze/one_jar`: inspect one deterministic fixture JAR.
- `analyze/300_jars`: analyze a generated 300-JAR corpus.
- `scan/300_jars`: scan the same 300-JAR workload.
- `pack/inspect_300_mods`: import and analyze a folder containing 300 JARs.
- `port/plan_and_dry_run_forge_1_16`: run the migration planner and migration engine in dry-run mode on the Forge 1.16 source fixture.
- `remap/jar_5_mib`: remap a generated 5 MiB JAR containing a fixture class and an uncompressed payload.
- `scan/5_mib`: scan a generated JAR containing a 5 MiB source entry.

The corpus is generated from checked-in fixture JARs at benchmark setup time. Fixture creation and copying are outside the timed sections. No network access or generated binary corpus is required.

## Running

From the repository root:

```sh
cargo bench -p ummp-core --bench phase13
```

Criterion output is written under `target/criterion`. Do not compare measurements across machines without recording the environment.

## Baseline (2026-11-23)

Environment: 2 vCPU Intel Xeon @ 2.10 GHz (cloud VM, 1 thread per core), 7 GiB RAM,
Linux 6.18, `rustc 1.97.0`, commit `a73a9ec` plus the benchmark fixes in this change.
Command: `cargo bench -p ummp-core --bench phase13`. Criterion lower / estimate / upper:

| Workload | Time | Throughput |
|---|---|---|
| `analyze/one_jar` | 8.74 / 9.60 / 10.59 µs | |
| `analyze/300_jars` | 8.69 / 9.21 / 9.82 ms | |
| `scan/300_jars` | 2.62 / 2.76 / 2.91 ms | |
| `pack/inspect_300_mods` | 146.6 / 152.3 / 158.5 ms | |
| `port/plan_and_dry_run_forge_1_16` | 3.25 / 3.37 / 3.51 ms | |
| `remap/jar_5_mib` | 2.78 / 2.84 / 2.90 ms | 1.72 GiB/s |
| `scan/jar_entry/5_mib` | 26.6 / 27.2 / 27.8 ms | 184 MiB/s |

The guide's example target (300-JAR analyze under 30 s on a 4-core laptop) is met by
three orders of magnitude, but the corpus is copies of tiny synthetic fixture JARs, so
these numbers measure UMMP's overhead, not real mods with thousands of classes. A real
large-mod benchmark stays open (BACKLOG "Heavy test mod").

Two benchmark bugs were fixed before these numbers could be taken: the port benchmark
copied into a directory it had not created, and the remap benchmark used the 11-byte
class header stub from `fabric-classes.jar`, which the remapper correctly rejects. It now
builds a valid synthetic `DemoWidget` class and asserts that exactly one class is renamed.
CI only compiles benchmarks, so neither bug was visible there.

## Deferred guide workloads

The benchmark targets cover all six workload categories in the guide. The 5 MiB remap fixture is generated from the checked-in class fixture and an uncompressed deterministic payload. Baseline measurements remain pending until the suite can be run on a development machine.
