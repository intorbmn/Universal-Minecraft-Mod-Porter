//! Phase 13A reproducible microbenchmarks.
use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::{tempdir, TempDir};
use ummp_core::analyzer::{inspect_jar, AnalyzerEngine};
use ummp_core::build::local::copy_project;
use ummp_core::bytecode::{remap_jar, ClassMap};
use ummp_core::checkpoint::CheckpointStore;
use ummp_core::filesystem::{FileSystem, LocalFileSystem};
use ummp_core::knowledge::KnowledgeBase;
use ummp_core::migration::{
    MigrationEngine, MigrationPlanner, MigrationRequest, RenameRule, RunOptions,
};
use ummp_core::modpack::analyze::{analyze_pack, AnalyzeOptions};
use ummp_core::modpack::import;
use ummp_core::project::{DefaultProjectManager, ProjectManager};
use ummp_core::scan::scan_path;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures")
}

fn fixture_jar() -> PathBuf {
    fixture_root().join("fabric-a.jar")
}

fn corpus_300() -> TempDir {
    let root = fixture_root();
    let jars: Vec<_> = fs::read_dir(&root)
        .expect("fixture directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "jar"))
        .collect();
    assert!(!jars.is_empty(), "fixture JAR corpus must not be empty");
    let dir = tempdir().expect("temporary corpus");
    for i in 0..300 {
        let source = &jars[i % jars.len()];
        fs::copy(source, dir.path().join(format!("mod-{i:03}.jar"))).expect("copy fixture JAR");
    }
    dir
}

fn five_mb_jar() -> (TempDir, PathBuf) {
    let dir = tempdir().expect("temporary JAR");
    let path = dir.path().join("five-mb.jar");
    let file = fs::File::create(&path).expect("create JAR");
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file(
        "src/main/java/Large.java",
        zip::write::SimpleFileOptions::default(),
    )
    .expect("start JAR entry");
    let chunk = "public class Large { String payload = \"benchmark-data\"; }\n";
    let mut remaining = 5 * 1024 * 1024;
    while remaining > 0 {
        let take = remaining.min(chunk.len());
        zip.write_all(&chunk.as_bytes()[..take])
            .expect("write JAR entry");
        remaining -= take;
    }
    zip.finish().expect("finish JAR");
    (dir, path)
}

fn bench_analyze_one(c: &mut Criterion) {
    let jar = fixture_jar();
    let fs: Arc<dyn FileSystem> = Arc::new(LocalFileSystem);
    c.bench_function("analyze/one_jar", |b| {
        b.iter(|| inspect_jar(black_box(&jar), fs.as_ref()).expect("analyze fixture JAR"));
    });
}

fn bench_analyze_300(c: &mut Criterion) {
    let corpus = corpus_300();
    let fs: Arc<dyn FileSystem> = Arc::new(LocalFileSystem);
    let engine = AnalyzerEngine::default();
    c.bench_function("analyze/300_jars", |b| {
        b.iter(|| {
            engine
                .analyze(black_box(corpus.path()), &fs)
                .expect("analyze corpus")
        });
    });
}

fn bench_scan_300(c: &mut Criterion) {
    let corpus = corpus_300();
    c.bench_function("scan/300_jars", |b| {
        b.iter(|| scan_path(black_box(corpus.path())).expect("scan corpus"));
    });
}

fn bench_pack_inspect_300(c: &mut Criterion) {
    let corpus = corpus_300();
    c.bench_function("pack/inspect_300_mods", |b| {
        b.iter(|| {
            let work = tempdir().expect("pack work directory");
            let mut pack = import(corpus.path(), work.path()).expect("import mod folder");
            analyze_pack(&mut pack, &AnalyzeOptions::default()).expect("analyze mod folder");
            black_box(pack.entries.len());
        });
    });
}

fn bench_plan_dry_run_port(c: &mut Criterion) {
    let source = fixture_root().join("forge-src-1.16");
    c.bench_function("port/plan_and_dry_run_forge_1_16", |b| {
        b.iter_batched(
            || {
                let temp = tempdir().expect("port workspace");
                let project_root = temp.path().join("project");
                // `copy_project` copies into an existing directory, as `port` prepares it.
                fs::create_dir_all(&project_root).expect("create port workspace");
                copy_project(&source, &project_root).expect("copy source fixture");
                let kb = KnowledgeBase::open(&temp.path().join("knowledge.db"))
                    .expect("open benchmark knowledge base");
                let project = DefaultProjectManager
                    .open(&project_root)
                    .expect("open benchmark project");
                let snapshots = temp.path().join("snapshots");
                (temp, kb, project, snapshots)
            },
            |(_temp, kb, project, snapshots)| {
                let plan = MigrationPlanner::plan(
                    &kb,
                    &MigrationRequest {
                        source_minecraft: "1.16.5".into(),
                        target_minecraft: "1.20.1".into(),
                        target_loader: "forge".into(),
                        min_confidence: 0.5,
                        source_loader: Some("forge".into()),
                    },
                )
                .expect("plan fixture port");
                let report = MigrationEngine::default()
                    .run(
                        &plan,
                        &project,
                        &mut CheckpointStore::new(),
                        &RunOptions {
                            dry_run: true,
                            snapshot_root: &snapshots,
                            session_id: uuid::Uuid::new_v4(),
                        },
                    )
                    .expect("dry-run fixture port");
                black_box(report.files_scanned)
            },
            BatchSize::SmallInput,
        );
    });
}

/// Minimal valid class `com/example/old/DemoWidget` (same layout as the bytecode unit-test
/// fixture). The checked-in `fabric-classes.jar` holds an 11-byte header stub that analysis
/// accepts but the remapper rightly rejects, so it cannot drive this benchmark.
fn demo_widget_class() -> Vec<u8> {
    let mut b = vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 52];
    let utf8 = |b: &mut Vec<u8>, s: &str| {
        b.push(1);
        b.extend_from_slice(&u16::try_from(s.len()).expect("short name").to_be_bytes());
        b.extend_from_slice(s.as_bytes());
    };
    b.extend_from_slice(&10u16.to_be_bytes());
    utf8(&mut b, "com/example/old/DemoWidget");
    b.extend_from_slice(&[7, 0, 1]);
    utf8(&mut b, "java/lang/Object");
    b.extend_from_slice(&[7, 0, 3]);
    b.extend_from_slice(&[5, 0, 0, 0, 0, 0, 0, 0, 42]);
    utf8(&mut b, "Lcom/example/old/DemoWidget;");
    utf8(&mut b, "self");
    utf8(&mut b, "com/example/old/DemoWidgetExtra");
    b.extend_from_slice(&[0, 0x21, 0, 2, 0, 4, 0, 0]);
    b.extend_from_slice(&[0, 1, 0, 0x02, 0, 8, 0, 7, 0, 0]);
    b.extend_from_slice(&[0, 0, 0, 0]);
    b
}

fn remap_fixture_5mb() -> (TempDir, PathBuf, Vec<ClassMap>) {
    let class_name = "com/example/old/DemoWidget.class".to_string();
    let class_bytes = demo_widget_class();
    let rule = RenameRule {
        from: "com.example.old.DemoWidget".into(),
        to: "benchmark.ported.DemoGadget".into(),
        kind: "class".into(),
        confidence: 1.0,
        version: "1.20.1".into(),
    };
    let stages = vec![ClassMap::from_rules(&[rule])];

    let dir = tempdir().expect("remap fixture directory");
    let input = dir.path().join("five-mib.jar");
    let mut jar = zip::ZipWriter::new(fs::File::create(&input).expect("create remap fixture"));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    jar.start_file(&class_name, options)
        .expect("write class entry");
    jar.write_all(&class_bytes).expect("write class bytes");
    jar.start_file("assets/benchmark/payload.bin", options)
        .expect("write payload entry");
    jar.write_all(&vec![b'x'; 5 * 1024 * 1024])
        .expect("write 5 MiB payload");
    jar.finish().expect("finish remap fixture");
    (dir, input, stages)
}

fn bench_remap_5mb(c: &mut Criterion) {
    let (_fixture_dir, input, stages) = remap_fixture_5mb();
    let check = tempdir().expect("remap check directory");
    let changed = remap_jar(&input, &check.path().join("remapped.jar"), &stages)
        .expect("remap benchmark JAR")
        .classes_changed;
    assert_eq!(
        changed, 1,
        "the benchmark must exercise a real class rename"
    );
    let mut group = c.benchmark_group("remap");
    group.throughput(criterion::Throughput::Bytes(5 * 1024 * 1024));
    group.bench_function("jar_5_mib", |b| {
        b.iter_batched(
            || tempdir().expect("remap output directory"),
            |output| {
                let result = remap_jar(&input, &output.path().join("remapped.jar"), &stages)
                    .expect("remap benchmark JAR");
                black_box(result.classes_changed)
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn bench_scan_5mb(c: &mut Criterion) {
    let (_dir, jar) = five_mb_jar();
    let mut group = c.benchmark_group("scan");
    group.throughput(criterion::Throughput::Bytes(5 * 1024 * 1024));
    group.bench_with_input(BenchmarkId::new("jar_entry", "5_mib"), &jar, |b, path| {
        b.iter(|| scan_path(black_box(path.as_path())).expect("scan 5 MiB JAR"));
    });
    group.finish();
}

criterion_group!(
    phase13,
    bench_analyze_one,
    bench_analyze_300,
    bench_scan_300,
    bench_pack_inspect_300,
    bench_plan_dry_run_port,
    bench_remap_5mb,
    bench_scan_5mb
);
criterion_main!(phase13);
