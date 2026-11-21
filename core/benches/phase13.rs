//! Phase 13A reproducible microbenchmarks.
use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::{tempdir, TempDir};
use ummp_core::analyzer::{inspect_jar, AnalyzerEngine};
use ummp_core::build::local::copy_project;
use ummp_core::checkpoint::CheckpointStore;
use ummp_core::filesystem::{FileSystem, LocalFileSystem};
use ummp_core::knowledge::KnowledgeBase;
use ummp_core::migration::{MigrationEngine, MigrationPlanner, MigrationRequest, RunOptions};
use ummp_core::modpack::analyze::{analyze_pack, AnalyzeOptions};
use ummp_core::modpack::{import, ImportOptions};
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
        fs::copy(source, dir.path().join(format!("mod-{i:03}.jar")))
            .expect("copy fixture JAR");
    }
    dir
}

fn five_mb_jar() -> (TempDir, PathBuf) {
    let dir = tempdir().expect("temporary JAR");
    let path = dir.path().join("five-mb.jar");
    let file = fs::File::create(&path).expect("create JAR");
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file("src/main/java/Large.java", zip::write::FileOptions::default())
        .expect("start JAR entry");
    let chunk = "public class Large { String payload = \"benchmark-data\"; }\n";
    let mut remaining = 5 * 1024 * 1024;
    while remaining > 0 {
        let take = remaining.min(chunk.len());
        zip.write_all(&chunk.as_bytes()[..take]).expect("write JAR entry");
        remaining -= take;
    }
    zip.finish().expect("finish JAR");
    (dir, path)
}

fn bench_analyze_one(c: &mut Criterion) {
    let jar = fixture_jar();
    let fs: Arc<dyn FileSystem> = Arc::new(LocalFileSystem);
    c.bench_function("analyze/one_jar", |b| {
        b.iter(|| inspect_jar(black_box(&jar), fs.as_ref()).expect("analyze fixture JAR"))
    });
}

fn bench_analyze_300(c: &mut Criterion) {
    let corpus = corpus_300();
    let fs: Arc<dyn FileSystem> = Arc::new(LocalFileSystem);
    let engine = AnalyzerEngine::default();
    c.bench_function("analyze/300_jars", |b| {
        b.iter(|| engine.analyze(black_box(corpus.path()), &fs).expect("analyze corpus"))
    });
}

fn bench_scan_300(c: &mut Criterion) {
    let corpus = corpus_300();
    c.bench_function("scan/300_jars", |b| {
        b.iter(|| scan_path(black_box(corpus.path())).expect("scan corpus"))
    });
}

fn bench_pack_inspect_300(c: &mut Criterion) {
    let corpus = corpus_300();
    c.bench_function("pack/inspect_300_mods", |b| {
        b.iter(|| {
            let work = tempdir().expect("pack work directory");
            let mut pack = import(corpus.path(), work.path()).expect("import mod folder");
            analyze_pack(&mut pack, &AnalyzeOptions::default()).expect("analyze mod folder");
            black_box(pack.entries.len())
        })
    });
}

fn bench_plan_dry_run_port(c: &mut Criterion) {
    let source = fixture_root().join("forge-src-1.16");
    c.bench_function("port/plan_and_dry_run_forge_1_16", |b| {
        b.iter_batched(
            || {
                let temp = tempdir().expect("port workspace");
                let project_root = temp.path().join("project");
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
        )
    });
}

fn bench_scan_5mb(c: &mut Criterion) {
    let (_dir, jar) = five_mb_jar();
    let mut group = c.benchmark_group("scan");
    group.throughput(criterion::Throughput::Bytes(5 * 1024 * 1024));
    group.bench_with_input(BenchmarkId::new("jar_entry", "5_mib"), &jar, |b, path| {
        b.iter(|| scan_path(black_box(path.as_path())).expect("scan 5 MiB JAR"))
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
    bench_scan_5mb
);
criterion_main!(phase13);
